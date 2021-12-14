vault client using jwt authentication that defines environment variables from vault secrets before
executing to something else

[TOC]

# Description

`envlt`, like [`env`](https://man7.org/linux/man-pages/man1/env.1.html), allows you to define environment variables
and then execute into something else, but instead of static values, it uses simple expressions to fetch secret
from a [vault server](https://www.vaultproject.io/) using a JWT Token as authentication.

It is useful in CI/CD environment, like [Gitlab](https://docs.gitlab.com/ee/ci/variables/) to securely access
secrets inside your jobs. It allows you to centralize secrets management by using the short lived JWT token
`CI_JOB_JWT` and get rid of all protected variables you normally define under Gitlab for that purpose. For more
complex cases involving services configuration, secret renewals, and restart of services, you will probably find
that [rconfd](https://github.com/eburghar/rconfd.git) is a better fit for the task.

Also gitlab premium user can [define vault
secrets](https://docs.gitlab.com/ee/ci/secrets/#use-vault-secrets-in-a-ci-job) directly in the project ci definition,
there is no such integrated mechanism for the community edition. You should in that case use the `vault` command, add
some boilerplate to login to the jwt service, get secrets one by one, then export them to environment variables. I
didn't fancy embedding the full vault executable which embed a client, an agent, and a server at the cost of 170MB,
only for that purpose and wanted the CI/CD jobs definitions to be as straightforward as possible. `envlt` integrate
all theses steps in one binary and never expose the secrets values in the command arguments contrary to a script.

# Usage

```
envlt 0.5.9

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
                    the same name is imported, when VALUE doesn't match an
                    expression with a backend, a new variable is defined with
                    the provided VALUE, otherwise the expression is expanded in
                    one or several variables and NAME is used as a prefix.
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

A variable expression following the `-V` flag has 2 form:

- `NAME`: try to get the `VALUE` from an environment variable with the same name (do nothing if not found)
- `NAME=VALUE`: define a new environment variable with a static value

If `VALUE` matches a backend expression `backend:args:path`, the value is expanded by calling the backend with
the provided arguments, and envlt define one variable name per leaf of the returned value, joining `NAME`
and the path components with `_`. Path components are keys for dictionaries and indexes (starting at 0) for arrays.

Otherwise, the variable is defined with the `VALUE`.

# Backends

There are currently 2 supported back-ends.

## Vault

```
vault:role[,GET|PUT|POST|LIST][,key=val]*:path[#json_pointer]
```

- `role` is the role name used for vault authentication,
- an optional http method that defaults to `GET`,
- optional keywords arguments that are sent as json dictionary in the body of the request,
- a path corresponding to the vault api point (without `/v1/`),
- an optional [json pointer](https://datatracker.ietf.org/doc/html/rfc6901) to define variables from. By default
  it is the root of the tree.

The vault secrets are cached by path (pointer excluded) and fetched only once. It is not really for
performance reason but because some api points generate different secret each time they are called like
[pki](https://www.vaultproject.io/docs/secrets/pki). You can define that way several variables with different
names (none is a prefix of the other) but tied to the same secret.

## Const

```
const:str|js:value
```

the value is parsed as json if `js` or kept as is if `str`

The main use of the `const:str:value` expression was to be able to differentiate a standard (not imported)
variable from one to be imported when using the `-I` flag, although you can achieve the same result in a more verbose
way by explicitly import a regular variable (whose value is not an expression) with `-V NAME`.

With `const:js` you can expand several static environment variables sharing a common prefix with one expression. If
you use [sccache](https://github.com/mozilla/sccache) with cargo you can use for example:

```
-V 'SCCACHE=const:js:{"bucket": "sccache", "endpoint": "minio:443", "s3_use_ssl": true}'
```

to define 3 variables

- `SCCACHE_BUCKET=sccache`
- `SCCACHE_ENDPOINT=minio:443`
- `SCCACHE_S3_USE_SSL=true`

to speedup compilation by using an S3 or compatible (minio) objects storage as build cache. You would also have
to provide access keys `AWS_*` which could come from a kv2 secret.

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

will add the following environment variables added to `command` context.

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

By default, `envlt` use a jwt token available in every kubernetes containers at
`/var/run/secrets/kubernetes.io/serviceaccount/token`. This token has claims about the kubernetes container
execution context you can use in vault to restrict the access to secrets.

You can also export the variables instead of defining them with `-V` and use `-I` option. This is useful in CI/CD
where you can define variables in the upper level, and hiding the details to keep the pipeline as simple as possible

```sh
export \
  'FOO=const:js:{"bar": 0, "baz": 1}' \
  BAR=const:str:3 \
  CERT=vault:role,POST,common_name=example.com:pki/issue/example.com \
  PACKAGER=vault:myrole:kv/data/secret#/data
envlt -I -V PATH -V HOME -- command args
```

If you choose not to import all the environment variables (you don't use `-i` flag along with `-I`) you can control
exactly which subset of variables are exported (the ones matching a backend expression) and add manually other
important regular variables like `PATH` or `HOME` like in the example above.

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
  - envlt -I -V PATH -V HOME -T CI_TOKEN_JWT -- build.sh
```
