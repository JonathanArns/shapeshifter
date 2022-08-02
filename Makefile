build:
	go build -o snake_proxy snake_proxy.go
	cargo build --release --features spl
deploy: build
	systemctl restart snake_proxy
	systemctl restart shapeshifter
