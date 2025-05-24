# Challenge Config Quickstart

All information about a challenge is configured in a `challenge.yaml` in the
challenge's directory in the repo, generally `<category>/<name>`.

For a tldr: see a full example for  [a TCP/`nc` challenge](#full-tcp-example) or [a web challenge](#full-http-example).

### Metadata

Self explanatory.

```yaml
name: yet another pyjail
author: somebody, John Author
```

### Description

Challenge description supports markdown and Jinja-style templating for challenge info.
The Jinja template fields available are:

| Field name  | Description |
| ----------- | ----------- |
| `hostname`  | The hostname or domain for the challenge
| `port`      | The port that the challenge is listening on
| `nc`        | Insert the `nc` command to connect to TCP challenges (`nc {{hostname}} {{port}}`)
| `link`      | Create a Markdown link to the exposed hostname/port
| `url`       | The URL from `link` without the accompanying Markdown
| `challenge` | The full challenge.yaml config for this challenge, with subfields

You probably only want `{{ nc }}` or `{{ link }}`.

Example:

```yaml
description: |
    Some example challenge. Blah blah blah flavor text.

    In case you missed it, this was written by {{ challenge.author }}
    and is called {{ challenge.name }}.

    {{ link }}    # -becomes-> [example.chals.thectf.com](https://example.chals.thectf.com)
    {{ nc }}      # -becomes-> `nc example.chals.thectf.com 12345`
```


### Flag

Read flag from file:

```yaml
flag:
  file: ./flag
```

### Pods

Defines how any container images for this challenge are built and deployed.

The pod `name` is also used for extracting files, see [Providing files to users](#Providing files to users).

`build` works similar to [Docker Compose](https://docs.docker.com/reference/compose-file/build/#illustrative-example),
either:
  - a string path to the build context folder
  - yaml with explicit `context` path, `dockerfile` path within context folder, and `args` build args \
    (only `context`, `dockerfile`, and `args` are supported for the detailed form)

`ports` controls how the container is exposed. This should be a list of what port the container is listening, and how
that port should be exposed to players:
- For TCP challenges, set `expose.tcp` to the subdomain and port: `<subdomain>:<port>`
- For HTTP challenges, set `expose.http` to the subdomain only: `<subdomain>` \
  The website domain will automatically be set up with an HTTPS cert.


```yaml
pods:
  - name: tcp-example
    build: .
    replicas: 2
    ports:
      - internal: 31337
        expose:
          tcp: thechal:30124  # exposed at thechal.<challenges_domain>:30124

  - name: web-example
    build:
      context: src/
      dockerfile: Containerfile
    replicas: 2
    ports:
      - internal: 31337
        expose:
          http: webchal  # exposed at https://webchal.<challenges_domain>
```




This can be omitted if there are no containers for the challenge.

### Providing files to users

Files to give to players as downloads in frontend.

These can be from the challenge folder in the repository, or from the
challenge's built container. These can also be zipped together into one file, or
uploaded separately. These need to be files, directories or globs are not (yet)
supported.

This can be omitted if there are no files provided.

```yaml
provide:
  # file from the challenge folder in the repo
  - somefile.txt

  # multiple files from src/ in the challenge folder, zipped as together.zip
  - as: together.zip
    include:
      - src/file1
      - src/file2
      - src/file3

  # multiple files pulled from the container image for the `main` pod
  # (see previous Pods section)
  - from: main
    include:
      - /chal/notsh
      - /lib/x86_64-linux-gnu/libc.so.6

  # same as above, but now zipped together
  - from: main
    as: notsh.zip
    include:
      - /chal/notsh
      - /lib/x86_64-linux-gnu/libc.so.6
```





# Examples

## Full TCP example

```yaml
name: notsh
author: John Author
description: |-
  This challenge isn't a shell

  {{ nc }}

provide:
  - from: main
    include:
      - /chal/notsh
      - /lib/x86_64-linux-gnu/libc.so.6

flag:
  file: ./flag

pods:
  - name: main
    build: .
    replicas: 2
    ports:
      - internal: 31337
        expose:
          tcp: 30124
```

## Full HTTP example

```yaml
name: bar
author: somebody
description: |
  can you order a drink from the webserver?

  {{ url }}

difficulty: 1

flag:
  file: ./flag

# no provide: section needed if no files

pods:
  - name: bar
    build:
      context: .
      dockerfile: Containerfile
    replicas: 1
    ports:
      - internal: 80
        expose:
          http: bar # subdomain only
```
