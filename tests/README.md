# Running Tests

Since this needs to interact with a container registry, S3 storage, and K8S,
there is some extra setup needed before running `cargo test` or running against
the test chals repo.

## Automatic setup

The integration tests in this directory will spin up local test containers as
part of the test process and set the corresponding envvar overrides. Since these
envvars are global, the tests are run serially to prevent each test from
competing to set envvars.

```
cargo test -j 1
```

These can be run in parallel using [cargo-nextest](https://nexte.st), which runs
each test in a separate process instead of threads. This allows each test to set
their own independent envvars without conflicts.

```
cargo nextest run
```

Some of these tests can be flaky when run with the default `cargo test` runner,
so using `nextest` is recommended if available.



## Manual test env

> [!NOTE]
> The tests have since been updated to create the required containers during the
> test setup. These scripts are kept around for interactive use but are no
> longer required.

### `setup.sh`

Main setup script. Run or source this file to set up infrastructure. Sourcing is
recommended to automatically set the config override environment environment
variables for test tokens and addresses.

Spins up a local K3D cluster and other test environment components via Docker
Compose.

```sh
source tests/setup.sh up
source tests/setup.sh down
```

### `services.compose.yaml`

Non-K8S resources required to run tests against:
  - Container registry
  - S3 buckets (via Minio)

### `repo/`

Example challenges repo to test against. Contains a variety of challenge types:
static file only (garf), HTTP web (bar), and TCP pwn (notsh).

This is also used by the integration tests above.
