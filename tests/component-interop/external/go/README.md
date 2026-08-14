# Go component provider fixture

`run.sh` validates a Go-produced component supplied in `ARUKELLT_GO_COMPONENT` and composes it through the manifest dependency path. Toolchain production is intentionally external so the gate does not vendor TinyGo/componentize-go.
