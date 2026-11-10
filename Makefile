PYTHON ?= python3

all:
	./combined/build-local.sh

test:
	./combined/build-local.sh --test-only
	$(PYTHON) -m unittest discover -s tests -p 'test_*.py' -v
	node tests/test_part_ui.cjs
	node tests/test_charge_ui.cjs
	node tests/test_timeline_ui.cjs

package:
	$(PYTHON) tools/package.py
	$(PYTHON) tools/package.py --source

.PHONY: all test package
