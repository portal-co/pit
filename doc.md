# Pit for WASM

## ABI v1

A unique ID should be unique across modules.

###  Exports

`pit/<resource id>/~<unique id>/<method>`

Drop methods are implemented as `pit/<resource id>/~<unique id>.drop`

### Imports

`pit/<resource id>`.`~<method>`

Drop methods are called as `pit`.`drop`