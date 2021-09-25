# envault

`envault` has mainly been developped to replace [Gitlab CI/CD variables](https://docs.gitlab.com/ee/ci/variables/),
to centralize secrets in vault and provide a more secure way to pass secrets to CI/CD jobs via a short-lived JWT
token (`$CI_JOB_JWT`). It can certainly be used in other contexts every time a JWT is available. For more complex
cases involving services configuration and secret renewals, [rconfd](https://github.com/eburghar/rconfd.git)
is certainly more suitable.

Also gitlab premium user can [define vault
secrets](https://docs.gitlab.com/ee/ci/secrets/#use-vault-secrets-in-a-ci-job) directly in the project ci definition,
there is not such integrated mechanism for the community edition. You should in that case use the `vault` client, add
some boilerplate to login to the jwt service, get secrets one by one, then export them to environment variables. I
didn't fancy embeding the full vault executable only for that purpose and wanted CI/CD jobs definitions to be as
straightforward as possible. `envault` integrate all theses steps in one binary and never expose the secrets
values in the command arguments contrary to a script.

In the spirit of the `env` command, `envault` modify the execution context by adding a series of enviroment
variables then replace itself with the command args.

## Usage

```
envault 0.1.0

Usage: envault <cmd> [<args...>] [-u <url>] [-j <jwt>] [-l <login-path>] [-v <vars...>]

Get vault secrets from path, modify environment, then executure args and command

Options:
  -u, --url         the vault url (https://localhost:8200)
  -j, --jwt         the env variable containing the JWT token (CI_JOB_JWT)
  -l, --login-path  the login path (/auth/jwt/login)
  -v, --vars        an expression NAME=PATH for defining a variable named NAME
                    from a vault path expression
  --help            display usage information
```

## Example

```sh
envault JSONCRT="gitlab-alpine,POST,common_name=example.com:pki/issue/example.com" \
        PACKAGER_KEY="gitlab-alpine:kv/data/abuild/key" \
        PACKAGER_PRIVKEY="gitlab-alpine,kv/data/abuild/keyid" \
        command args
```

## Configuring vault

activate Vault jwt authentication

```sh
vault write auth/jwt/config jwks_url="https://gitlab.com/-/jwks" bound_issuer="gitlab.com"
```

create a policy for accessing the secrets

```sh
vault policy write gitlab-runner - <<EOF
path "kv/data/abuild/*" {
  capabilities = [ "read" ]
}
path "pki/issue/example.com" {
  capabilities = [ "read", "create", "update" ]
}
EOF```

create a role

```sh
vault write auth/jwt/role/gitlab-runner - <<EOF
{
  "role_type": "jwt",
  "policies": ["gitlab-runner"],
  "token_explicit_max_ttl": 60,
  "user_claim": "user_email",
  "bound_claims": {
    "group_path": "alpine",
    "ref_protected": "true",
    "ref_type": "tag"
  }
}
```
