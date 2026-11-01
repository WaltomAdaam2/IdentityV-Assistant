import json
import logging

from main import RuntimeLogFormatter


def test_window_debug_and_traceback_are_one_complete_protocol_record():
    formatter = RuntimeLogFormatter()
    record = logging.LogRecord("window.automation", logging.DEBUG, __file__, 1, "trace\n" * 1000, (), None)
    encoded = formatter.format(record)
    assert len(encoded.splitlines()) == 1
    value = json.loads(encoded)
    assert value["message"] == "trace\n" * 1000
    assert value["level"] == "debug"
    assert value["data"]["important"] is False
    try:
        raise ValueError("window fixture")
    except ValueError as error:
        record.levelno = logging.WARNING
        record.levelname = "WARNING"
        record.exc_info = (type(error), error, error.__traceback__)
    assert "ValueError: window fixture" in json.loads(formatter.format(record))["message"]
