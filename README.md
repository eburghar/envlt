# envlt

`envlt`, like [`env`](https://man7.org/linux/man-pages/man1/env.1.html), allows you to define environment variables
and then execute into something else, but instead of static values, it uses using simple expressions to fetch secret
from a [vault server](https://www.vaultproject.io/) using a JWT Token as authentication.

It is useful in CI/CD environment, like [Gitlab](https://docs.gitlab.com/ee/ci/variables/) to securely access
secrets inside your jobs. It allows you to centralize secrets management by using the short lived JWT token
`CI_JOB_JWT` and get rid of all static variables you normally define under Gitlab for that purpose. For more
complex cases involving services configuration, secret renewals, and restart of services, you will probably find
that [rconfd](https://github.com/eburghar/rconfd.git) is a better fit for the task.

Also gitlab premium user can [define vault
secrets](https://docs.gitlab.com/ee/ci/secrets/#use-vault-secrets-in-a-ci-job) directly in the project ci definition,
there is no such integrated mechanism for the community edition. You should in that case use the `vault` command, add
some boilerplate to login to the jwt service, get secrets one by one, then export them to environment variables. I
didn't fancy embedding the full vault executable only for that purpose and wanted CI/CD jobs definitions to be as
straightforward as possible. `envlt` integrate all theses steps in one binary and never expose the secrets
values in the command arguments contrary to a script.

## Usage

```
envlt 0.5.0

Usage: envlt <cmd> [<args...>] [-u <url>] [-l <login-path>] [-c <cacert>] [-T <token>] [-t <token-path>] [-V <vars...>] [-v] [-i] [-I]

Get vault secrets from path expressions, define environment variables, then execute into args and command

Options:
  -u, --url         the vault url ($VAULT_URL or https://localhost:8200/v1)
  -l, --login-path  the login path (/auth/jwt/login)
  -c, --cacert      path of vault CA certificate
                    (/var/run/secrets/kubernetes.io/serviceaccount/ca.crt)
  -T, --token       the JWT token taken from the given variable name or from the
                    given string if it fails (takes precedence over -t)
  -t, --token-path  path of the JWT token
                    (/var/run/secrets/kubernetes.io/serviceaccount/token)
  -V, --vars        an expression NAME[=VALUE] for defining one or several
                    variables. When no VALUE given, an environment variable with
                    the same name is imported, when VALUE doesn't match a
                    expression, a new variable is defined with the provided
                    VALUE, otherwise the expression is expanded in one or
                    several variables and NAME is used as a prefix.
  -v, --verbose     verbose mode
  -i, --import      import all environment variables before executing into cmd
  -I, --import-vault
                    import environment variables whose values matches a
                    vault_path a whose expansion is successful
  --help            display usage information
```

By default, `envlt` starts with an empty context, meaning that no variables are exposed to `cmd`. There is 3 options to
alter this behavior you can mix together:

- `-i` import all accessible variables "as is"
- `-I` import only the variables that match an expression with a backend
- `-V` (re)define variables (takes precedence over `-i` and `-I`) or import existing ones

# Variable expression

A variable expression following the `-V` flag has 3 form:

- `NAME`: import an environment variable with the same name
- `NAME=VALUE`: define a new environment variable with a static value
- `PREFIX=backend:args:path[#anchor]`: define one or several variables by fetching their value from a backend. When
  the returned value is structured (`vault` backend and `const` backend with `js` value), envlt recursively define one
  variable name for each leaf of the json tree by joining the prefix and path components with `_`. Path components
  are keys for dictionaries and indexes (starting at 0) for arrays.

# Backends

There are currently 2 supported back-ends.

## Vault

```
vault:role[,GET|PUT|POST|LIST][,key=val]*:path[#json_pointer]
```

- `role` is the role name used for vault authentication,
- an optional http method that defaults to `GET`
- optional keywords arguments that are sent as json dictionary in the body of the request
- a path corresponding to the vault api point (without `/v1/`)
- an optional [json pointer](https://datatracker.ietf.org/doc/html/rfc6901) to define variables from and that defaults
  to the root of the tree.

The vault secrets are cached by path (pointer excluded) and fetched only once. It is not really for
performance reason but because some api points generate different secret each time they are called like
[pki](https://www.vaultproject.io/docs/secrets/pki). You can define that way several variables with different
prefixes but tied to the same secret.

## Const

```
const:str|js:value
```

the value is parsed as json if `js` or kept as is if `str`

The main use of the `const:str:value` expression was to be able to differentiate a standard (not imported)
variable from one to be imported when using the `-I` flag, although you can achieve the same result in a more verbose
way by explicitly import a regular (whose value is not an expression) variable with `-V NAME`.

# Example

If you have a [pki](https://www.vaultproject.io/docs/secrets/pki) backend mounted at `/pki`, and a
[kv2](https://www.vaultproject.io/docs/secrets/kv/kv-v2secret) secret defined at `kv/abuild` with the following content

```yaml
crt: xxxx
key: xxxx
keyid: xxxx
```

calling `envlt` with the following arguments

```sh
envlt -V 'FOO=const:js:{"bar": 0, "baz": 1}'
      -V BAR=3
      -V CERT=vault:role,POST,common_name=example.com:pki/issue/example.com \
      -V PACKAGER=vault:myrole:kv/data/secret#/data
      -- command args
```

By default, `envlt` use a jwt token available in every kubernetes containers at
`/var/run/secrets/kubernetes.io/serviceaccount/token`. This token has claims about the kubernetes container
execution context you can use to restrict the access to secrets. Here, `command args` will have the following
environment variables added to its context.

- FOO_BAR=0
- FOO_BAZ=1
- BAR=3
- CERT_CERTIFICATE=...
- CERT_ISSUING_CA=...
- CERT_CA_CHAIN_0=...
- ...
- CERT_CA_CHAIN_n=...
- CERT_PRIVATE_KEY=...
- CERT_PRIVATE_KEY_TYPE=...
- CERT_SERIAL_NUMBER=...
- PACKAGER_CRT=...
- PACKAGER_KEY=...
- PACKAGER_KEYID=...

You can also export the variables, and use `-I` option. This is useful in CI/CD where you can define variables
in the upper level, and hiding the details to keep the pipeline as simple as possible

```sh
export \
  'FOO=const:js:{"bar": 0, "baz": 1}' \
  BAR=const:str:3 \
  CERT=vault:role,POST,common_name=example.com:pki/issue/example.com \
  PACKAGER=vault:myrole:kv/data/secret#/data
envlt -I -V PATH -V HOME -- command args
```

If you choose not to import all the environment variables (`-i`), you may have to manually import some important ones
like `PATH` or `HOME` like in the example above.

# Using envlt with Gitlab CI/CD

## Configuring vault

Activate vault jwt authentication

```sh
vault write auth/jwt/config jwks_url="https://gitlab.com/-/jwks" bound_issuer="gitlab.com"
```

Create a policy for accessing the secrets

```sh
vault policy write mypolicy - <<EOF
path "kv/data/secrets/*" {
  capabilities = [ "read" ]
}
EOF
```

Create a role. Here You can only login with `role`, and only a build on a project inside gitlab group `mygroup`
and for a protected tag (release) will have access to the secret.

```sh
vault write auth/jwt/role/myrole - <<EOF
{
  "role_type": "jwt",
  "policies": ["mypolicy"],
  "token_explicit_max_ttl": 60,
  "user_claim": "user_email",
  "bound_claims": {
    "group_path": "mygroup",
    "ref_protected": "true",
    "ref_type": "tag"
  }
}
```

## Configuring Gitlab CI/CD

You should make a build image (`mybuilder`) containing the envlt executable. Then you just have to call `envlt`
in your pipelines script using the JWT token from the environment variable `CI_JOB_JWT` (note that we use a
variable name here instead of a substitution to not expose the token on command line arguments)

You must define a `VAULT_URL=const:str:https://localhost:8200` and a `SECRET=vault:myrole:kv/data/secrets#/data`
variables and a good place for that is in the project or group settings.

Here is an example `.gitlab-ci.yml`

```yaml
image: mybuilder

build:
  stage: build
  script:
  # The Makefile use files containing secrets generated by rconfd
  - envlt -I -V PATH -V HOME -T CI_TOKEN_JWT -- make
```
