CC=gcc
C_FLAGS=-Wall # disable NX: -z execstack 
              # disable canary: -fno-stack-protector
			  # disable PIE: -no-pie
C_LIBS= # -lcrypto or something

out=notsh

.PHONY: all
all: $(out)

$(out): src/*.c
	$(CC) $(C_FLAGS) -o $@ $^ $(C_LIBS)

# container builds this target
# make sure 'all' builds everything you need
# container builds on ubu1804
.PHONY: container
container: all

.PHONY: clean
clean:
	$(RM) $(out) *.o
