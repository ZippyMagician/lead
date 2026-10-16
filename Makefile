.PHONY: ffi
.DEFAULT: ffi
ffi:
	cargo build --release --features "ffi"
	cp lead/c/liblead.h build/
	gcc -fPIC -shared build/liblead.h build/leadrustbindings.h -o build/liblead.so
