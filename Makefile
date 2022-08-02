build:
	go build -o snake_proxy snake_proxy.go
	cargo build --release --features spl
