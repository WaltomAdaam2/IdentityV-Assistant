"""Retired entry point: module hotfixes are never downloaded or applied.

Old helper invocations fail closed; the assistant now installs official full releases.
"""
if __name__ == "__main__":
    raise SystemExit("Module hotfixes are disabled. Use the assistant's official full component update.")
