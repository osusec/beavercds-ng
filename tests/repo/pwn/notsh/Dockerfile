# IMAGE 1: build challenge
# @AUTHOR: if your chal doesn't build seperately from being run (i.e. Python),
#          delete all of the IMAGE 1 code
FROM ubuntu:18.04 AS builder

# @AUTHOR: build requirements here
RUN apt-get -qq update && apt-get -qq --no-install-recommends install build-essential

WORKDIR /build

# @AUTHOR: make sure all source is copied in. If everything is in src/, no change needed
COPY src ./src/
COPY Makefile .
RUN make container

# IMAGE 2: run challenge
# @AUTHOR: feel free to change base image as necessary (i.e. python, node)
FROM ubuntu:18.04

# @AUTHOR: run requirements here
RUN apt-get -qq update && apt-get -qq --no-install-recommends install xinetd

# copy binary
WORKDIR /chal
# @AUTHOR: make sure all build outputs are copied to the runner
#          if there is no build output, replace this with the appropriate COPY stmts
#          to pull files from the host
COPY --from=builder /build/notsh /chal/

# copy flag
COPY flag /chal/

# make user
RUN useradd chal

# copy service info
COPY container_src/* /

# run challenge
EXPOSE 31337
RUN chmod +x /run_chal.sh
CMD ["/usr/sbin/xinetd", "-syslog", "local0", "-dontfork", "-f", "/xinetd.conf"]
