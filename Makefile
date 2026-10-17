WASM_AD ?= ../wasm-ad
PORT ?= 8642
WASM := target/wasm32-unknown-unknown/release/rainbow_river.wasm
CART := web/carts/rainbow-river.wasm

.PHONY: build player serve test clean

# The cart: a WASM-AD .wasm module.
build:
	cargo build --release --target wasm32-unknown-unknown -p rainbow-river
	@mkdir -p web/carts
	@if command -v wasm-opt >/dev/null; then \
		wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int $(WASM) -o $(CART); \
	else cp $(WASM) $(CART); fi
	@ls -l $(CART) | awk '{print "$(CART): " $$5 " bytes"}'

# The WASM-AD web player, copied from a wasm-ad checkout.
player:
	$(MAKE) -C $(WASM_AD) player
	cp $(WASM_AD)/player/web/index.html $(WASM_AD)/player/web/player.js $(WASM_AD)/player/web/core.wasm web/

serve: build player
	@echo "http://localhost:$(PORT)/?cart=carts/rainbow-river.wasm"
	python3 -m http.server $(PORT) --directory web

test:
	cargo test

clean:
	cargo clean
	rm -rf web
