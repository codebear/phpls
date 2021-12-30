all: 
	@echo make run
	@echo make phpdefs

run:
	cargo run

devrun: phpdefs run

phpdefs:
	cd php && php generate_ast_nodes.php > astnodes.rs
	cd php && php generate_ast_map.php > astmetadata.rs

dumpserializedast:
	 php parse.php

ROOT=$(shell pwd)/..
test:
	cd ~/src/cerum && $(ROOT)/target/debug/phplint --describe src/lib/ew/info/contact.class.php 45 30