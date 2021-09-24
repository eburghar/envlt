# envault

`envault` has mainly been developped to replace [Gitlab CI/CD variables](https://docs.gitlab.com/ee/ci/variables/),
to centralize secrets in vault and provide a more secure way to pass secrets to CI/CD jobs via a short-lived
JWT token (`$CI_JOB_JWT`). It can certainly be used in other contexts, every time a JWT is available
to access vault secrets. For more complex cases involving services configuration and secret renewals,
[rconfd](https://github.com/eburghar/rconfd.git) is certainly more suitable.

Also gitlab premium user can [define vault
secrets](https://docs.gitlab.com/ee/ci/secrets/#use-vault-secrets-in-a-ci-job) directly in the project ci
definition, there is not such integrated mechanism for the community edition. You should in that case use the
`vault` client, add some boilerplate to login to the jwt service, get secrets one by one, then export them to
environment variables. `envault` by integrating all theses steps in on binary is more secure as the variables
contents are never exposed in the command arguments contrary to a script. I didn't fancy embeding the full vault
executable only for that purpose and wanted CI/CD jobs definitions to be as straightforward as possible.

In the spirit of the `env` command, `envault` modify the execution context by adding a series of enviroment
variables then replace itself with the command args.

```sh
envault PACKAGER_CRT="role:kv/data/abuild/crt" \
        PACKAGER_KEY="role:kv/data/abuild/key" \
        PACKAGER_PRIVKEY="role:kv/data/abuild/keyid" \
        command args
```

## Configuring vault

activate Vault jwt authentication

```sh
vault write auth/jwt/config jwks_url="https://gitlab.com/-/jwks" bound_issuer="gitlab.com"
```

create a policy for accessing the secrets

```sh
```

create a role

```sh
```
