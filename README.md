# envlt

`envlt` has mainly been developped to replace [Gitlab CI/CD variables](https://docs.gitlab.com/ee/ci/variables/)
whenever possible to centralize secrets in vault and provide a more secure way to pass secrets to CI/CD
jobs via a short-lived JWT token (`$CI_JOB_JWT`). It can certainly be used in other contexts every
time a JWT is available. For more complex cases involving services configuration and secret renewals,
[rconfd](https://github.com/eburghar/rconfd.git) can be more suitable.

Also gitlab premium user can [define vault
secrets](https://docs.gitlab.com/ee/ci/secrets/#use-vault-secrets-in-a-ci-job) directly in the project ci definition,
there is no such integrated mechanism for the community edition. You should in that case use the `vault` command, add
some boilerplate to login to the jwt service, get secrets one by one, then export them to environment variables. I
didn't fancy embeding the full vault executable only for that purpose and wanted CI/CD jobs definitions to be as
straightforward as possible. `envlt` integrate all theses steps in one binary and never expose the secrets
values in the command arguments contrary to a script.

In the spirit of the `env` command, `envlt` replace itself with the command and args given as paramaters after adding
environment variable to its execution context. As vault secrets are structured, envlt recursively define environment names
by appending a prefix to a path whose components are separated by `_`. Path components are keys for dictionaries and
indexes (starting at 0) for arrays.

## Usage

```envlt 0.1.0

Usage: envlt <cmd> [<args...>] [-u <url>] [-l <login-path>] [-c <cacert>] [-T <token>] [-t <token-path>] [-V <vars...>] [-v] [-i] [-I]

Get vault secrets from path expressions, define environment variables, then execute into args and command

Options:
  -u, --url         the vault url ($VAULT_URL or https://localhost:8200/v1)
  -l, --login-path  the login path (/auth/jwt/login)
  -c, --cacert      path of vault CA certificate
                    (/var/run/secrets/kubernetes.io/serviceaccount/ca.crt)
  -T, --token       the JWT token taken from the given variable name or from the
                    given string if it fails (take precedence over -t)
  -t, --token-path  path of the JWT token
                    (/var/run/secrets/kubernetes.io/serviceaccount/token)
  -V, --vars        an expression PREFIX=PATH for defining one or several
                    variables prefixed with PREFIX from a vault path expression
  -v, --verbose     verbose mode
  -i, --import      import all environment variables before executing into cmd
  -I, --import-vault
                    import environment variables whose values matches a
                    vault_path a whose expansion is successful
  --help            display usage information
```

# Vault path expression

a Vault path expression has the following structure :

```
role[,GET|PUT|POST|LIST][,key=val]*:path:json_pointer
```

- `role` is the role name used for vault authentication,
- the optional http method defaults to `GET`
- the optional keywords arguments are sent as json dictionary in the body of the request
- a path corresponding to the vault api point (without `/v1/`)
- the [json pointer](https://datatracker.ietf.org/doc/html/rfc6901) is for a path into the secret json structure to
  construct the list of variable names from

The vault secrets are cached by path and fetched only once, so you can define 2 different variables using the same
secret but a different json pointer. It is a key feature for api points that generate a different secret each time they
are called like [pki](https://www.vaultproject.io/docs/secrets/pki), to keep different part of the same secret in sync.

```sh
envlt -V CRT="role,POST,common_name=example.com:pki/issue/example.com:/certificate" \
      -V KEY="role,POST,common_name=example.com:pki/issue/example.com:/private_key" \
      command args
```

## Example

If you have a [kv2](https://www.vaultproject.io/docs/secrets/kv/kv-v2secret) defined at `kv/abuild` with the
following content

```yaml
crt: xxxx
key: xxxx
keyid: xxxx
```

```sh
envlt -V CERT="role,POST,common_name=example.com:pki/issue/example.com:" \
      -V PACKAGER="role:kv/data/secret:/data"
      command args
```

will call command args with the following enviroment variables added to context
- CERT_CERTIFICATE
- CERT_ISSUING_CA
- CERT_CA_CHAIN_0
- ...
- CERT_CA_CHAIN_n
- CERT_PRIVATE_KEY
- CERT_PRIVATE_KEY_TYPE
- CERT_SERIAL_NUMBER
- PACKAGER_CRT
- PACKAGER_KEY
- PACKAGER_KEYID

You can also export the variables, and use `-I` option. This is useful in CI/CD where you can define variables
in the upper level, and keep the pipeline simple

```sh
export CERT="role,POST,common_name=example.com:pki/issue/example.com:" \
      PACKAGER="role:kv/data/secret:/data"
envlt -I command args
```

# Using rconfd with Gitlab CI/CD

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

Create a role. Here You can only login with that role if the project is inside the `alpine` group and the
build is for a protected tag (release).

```sh
vault write auth/jwt/role/myrole - <<EOF
{
  "role_type": "jwt",
  "policies": ["mypolicy"],
  "token_explicit_max_ttl": 60,
  "user_claim": "user_email",
  "bound_claims": {
    "group_path": "alpine",
    "ref_protected": "true",
    "ref_type": "tag"
  }
}
```

## Configuring Gitlab CI/CD

You should make a build image (`mybuilder`) containing the envlt executable. Then you just have to call envlt in your
pipelines script reading the JWT token from the environment variable `CI_JOB_JWT` (note that we use a variable name
here and not a substitution to not expose the token on the command line arguments)

You must define a `VAULT_URL` and a `PACKAGER=role:kv/data/secrets:/data` variables and a good place for that is
in the project or group settings.

Here is an example `.gitlab-ci.yml`

```yaml
image: mybuilder

build:
  stage: build
  script:
  # The Makefile use files containing secrets generated by rconfd
  - envlt -T CI_TOKEN_JWT -I make
```
