# Running Tests

Since this needs to interact with a container registry, S3 storage, and K8S,
there is some extra setup needed before running `cargo test` or running against
the test chals repo.

## `setup.sh`

Main setup script. Run or source this file to set up infrastructure.
Recommended to source this file to set the config override environment
environment variables for test tokens and addresses.

Spins up a local Minikube K8S cluster and other test environment components via
Docker Compose.

```sh
source tests/setup.sh up
source tests/setup.sh down
```

## `services.compose.yaml`

Non-K8S resources required to run tests against:
  - Container registry
  - S3 buckets (via Minio)

## `repo/`

Example challenges repo to test against. Contains a variety of challenge types:
static file only (garf), HTTP web (bar), and TCP pwn (notsh).
