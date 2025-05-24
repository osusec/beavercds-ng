# Challenge Config Reference

Challenge configuration is expected to be at `<category>/<name>/challenge.yaml`.

[[toc]]

## `name`

The name of the challenge, as shown to players in the frontend UI.

```yaml
name: notsh

# can have spaces:
name: Revenge of the FIPS
```

## `author`

Author or authors of the challenge, as shown to players in the frontend UI. If there are multiple authors, specify them as one string.

```yaml
author: John Author

# multiple authors:
author: Alice, Bob, and others
```

## `description`

Description and flavortext for the challenge, as shown to players in the frontend UI. Supports templating to include information about the challenge, such as the link or command to connect.

Most challenges only need `{{ nc }}` or `{{ link }}`.

| Available fields | Description                                                                                                              |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `domain`         | Full domain the challenge is exposed at, e.g. `<subdomain>.chals.example.ctf`                                            |
| `port`           | Port the challenge is listening on                                                                                       |
| `nc`             | `nc` command to connect to TCP challenges, with Markdown backticks <br> (equivalent to `` `nc {{domain}} {{port}}` ``)   |
| `url`            | URL to the exposed web domain for web challenges, plus port if needed <br> (equivalent to `https://{{domain}}:{{port}}`) |
| `link`           | Markdown link to `url`                                                                                                   |
| `challenge`      | The full challenge.yaml config object for this challenge, with subfields                                                 |

```yaml
description: |
  Some example challenge. Blah blah blah flavor text.

  In case you missed it, this was written by {{ challenge.author }}
  and is called {{ challenge.name }}.

  {{ nc }}      # `nc somechal.chals.example.ctf 12345`
  {{ link }}    # [https://somechal.chals.example.ctf](https://somechal.chals.example.ctf)
```

## `category`

The category for the challenge.

::: warning
This is set from the expected directory structure of `<category>/<challenge>/challenge.yaml` and will overwrite whatever is set in the file.
:::

## `difficulty`

::: info
Not implemented yet, does nothing
:::

The difficulty from the challenge, used to set point values. Values correspond to entries in the [rcds.yaml difficulty settings](../for-sysadmins/config#difficulty).

```yaml
difficulty: 1 # the current default
```

## `flag`

Where to find the flag for the challenge. The flag can be in a file, a regex, or a direct string.

```yaml
# directly set
flag: ctf{example-flag}

# from a file in in the challenge directory
flag:
  file: ./flag

# regex
flag:
  regex: /ctf\{(foo|bar|ba[xyz])\}/
```

::: info
Regex flags are not implemented yet and setting one does nothing
:::

## `provide`

List of files to provide to the players on the frontend UI. These files can be from the challenge directory or from a container image built for a [challenge pod](#pods), and uploaded individually or zipped together.

If there are no files to upload for this challenge, this can be omitted or set to an empty array.

```yaml
provide:
  # files from the challenge folder in the repo
  - somefile.txt
  - otherfile.txt

  # these are all equivalent
  - foo.txt
  - include: foo.txt
  - include: [ foo.txt ]

  # rename a really long name as something shorter for upload
  - as: short.h
    include: some_really_long_name.h

  # multiple files from src/ in the challenge folder, zipped as together.zip
  - as: together.zip
    include:
      - src/file1
      - src/file2
      - src/file3

  # multiple files pulled from the container image for the `main` pod,
  # uploaded individually as `notsh` and `libc.so.6`
  - from: main
    include:
      - /chal/notsh
      - /lib/x86_64-linux-gnu/libc.so.6

  # single file pulled from the main container and renamed
  - from: main
    as: libc.so
    include: /lib/x86_64-linux-gnu/libc.so.6

  # multiple files pulled from the main container and zipped together
  - from: main
    as: notsh.zip
    include:
      - /chal/notsh
      - /lib/x86_64-linux-gnu/libc.so.6


# if no files need to be provided:
provide: []
# or omit entirely
```

### `.include`

File or list of files to upload individually, or include in a zip if `as` is set.

When uploading, only the basename is used and the path to the file is discarded.

If a provide item is specified as a single string, it is interpreted as an `include:`.

### `.as`

If `.include` is a single file, rename to this name while uploading.

If multiple files, zip them together into the given zip file.

### `.from`

Fetch these files from the corresponding [challenge pod](#pods) image.

## `pods`

Defines how to build and deploy any services needed for the challenge.

Challenge pods can be built from a local Dockerfile in the challenge folder or use an upstream image directly.

If there are no pods or images needed for this challenge, this can be omitted or set to an empty array.

```yaml
pods:
  - name: main
    build: .
    ports:
      - internal: 1337        # expose a container listening on port 1337 ...
        expose:
          http: examplechal   # as a web chal at https://examplechal.<chals_domain>

  - name: db
    image: postgres:alpine
    architecture: arm64
    env:
      POSTGRES_USER: someuser
      POSTGRES_PASSWORD: notsecure

# if no containers or pods need to be deployed:
pods: []
# or omit entirely
```

### `.name`

Name of the pod, used to refer to this container as [a source for `provide` files](#provide) and for generated resource names.

Cannot contain spaces or punctuation, only alphanumeric and `-`.

### `.build`

Build the container image for this pod from a local `Dockerfile`. Supports a subset of the [docker-compose build spec](https://docs.docker.com/reference/compose-file/build/#illustrative-example) (`dockerfile`, `context`, `args`).

Conflicts with [`image`](#image).

```yaml
    # build a container from a Dockerfile in the challenge folder
    build: .

    # equivalent to the above but with explicit build context and Dockerfile name
    build:
      context: .
      dockerfile: Dockerfile

    # build from a subfolder with a custom Dockerfile and some build args
    build:
      context: src/
      dockerfile: Containerfile.remote
      args:
        CC_OPTS: "-Osize"
```

### `.image`

Use an available container image for the pod instead of building one from source.

Conflicts with [`build`](#build).

### `.env`

Any environment variables to set for the running pod. Specify as `name: value`.

```yaml
env:
  SOME_ENVVAR: foo bar
```

### `.architecture`

Set the desired CPU architecture to run this pod on.

```yaml
    architecture: amd64   # AKA x86_64; the default
    architecture: arm64   # for ARM
```

### `.resources`

The resource usage request and limits for the pod. Kubernetes will make sure the requested resources will be available for this pod to use, and will also restart the pod if it goes over these limits.

If not set, the default set in [`rcds.yaml`](../for-sysadmins/config#rcds.yaml) is used.

### `.replicas`

How many instances of the pod to run. Traffic is load-balanced between instances.

Default is 2 and this is probably fine unless the challenge is very resource intensive.

```yaml
replicas: 2 # the default
```

### `.ports`

Specfies how to expose this pod to players, either as a raw TCP port or HTTP at a specific domain.

#### `.ports.internal`

The port the container is listening on; i.e. `xinetd` or `nginx` etc.

#### `.ports.expose`

How to expose the internal container port

### `.volume`
