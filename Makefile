.PHONY: fmt
fmt:
	cargo fmt

.PHONY: clippy
clippy:
	cargo clippy

.PHONY: test 
test:
	cargo test 

.PHONY: unused-dependencies 
unused-dependencies:
	cargo machete

.PHONY: check 
check: fmt clippy test unused-dependencies
