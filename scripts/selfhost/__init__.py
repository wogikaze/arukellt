"""Selfhost package wiring."""

# ``scripts/manager.py`` imports run_fixture_parity from selfhost.checks. Keep
# that stable entrypoint while replacing its semantics atomically with the new
# current-only parallel fixture gate. The historical implementation is captured
# inside fixture_parity.py and is reachable only through the explicit bootstrap
# reference runner.
from . import checks as _checks
from .fixture_parity import run_fixture_parity as _run_fixture_parity

_checks.run_fixture_parity = _run_fixture_parity
