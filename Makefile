WASM_AD ?= ../wasm-ad
PORT ?= 8642
WASM := target/wasm32-unknown-unknown/release/rainbow_river.wasm
CART := web/carts/rainbow-river.wasm

DIST := dist
SITE_PORT ?= 8645

.PHONY: build player serve site site-serve test clean

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
	cp $(WASM_AD)/player/web/index.html $(WASM_AD)/player/web/player.js $(WASM_AD)/player/web/wasm-ad.js $(WASM_AD)/player/web/core.wasm web/

serve: build player
	@echo "http://localhost:$(PORT)/?cart=carts/rainbow-river.wasm"
	python3 -m http.server $(PORT) --directory web

# The website: site/ plus the WASM-AD player and the cart, in dist/.
site: build
	$(MAKE) -C $(WASM_AD) player
	rm -rf $(DIST)
	mkdir -p $(DIST)/play $(DIST)/carts
	cp -R site/. $(DIST)/
	cp $(WASM_AD)/player/web/index.html $(WASM_AD)/player/web/player.js $(WASM_AD)/player/web/wasm-ad.js $(WASM_AD)/player/web/core.wasm $(DIST)/play/
	cp $(CART) $(DIST)/carts/

site-serve: site
	@echo "http://localhost:$(SITE_PORT)/"
	python3 -m http.server $(SITE_PORT) --directory $(DIST)

test:
	cargo test

clean:
	cargo clean
	rm -rf web $(DIST)
