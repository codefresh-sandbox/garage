BIN=target/release/garage
DOCKER=lxpz/garage_amd64

all:
	clear; RUSTFLAGS="-C link-arg=-fuse-ld=lld -C target-cpu=x86-64 -C target-feature=+sse2" cargo build --no-default-features

$(BIN):
	RUSTFLAGS="-C link-arg=-fuse-ld=lld -C target-cpu=x86-64 -C target-feature=+sse2" cargo build --release --no-default-features

$(BIN).stripped: $(BIN)
	cp $^ $@
	strip $@

docker: $(BIN).stripped
	docker build -t $(DOCKER):$(TAG) .
	docker push $(DOCKER):$(TAG)
	docker tag $(DOCKER):$(TAG) $(DOCKER):latest
	docker push $(DOCKER):latest
	
