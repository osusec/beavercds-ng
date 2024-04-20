build:
	go build -o out/beavercds src/main.go

run:
	go run src/main.go ${ARGS}

clean:
	rm -f out/beavercds
