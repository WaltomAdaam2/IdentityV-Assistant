import builtins

import pytest

from vision.ocr import RapidOcrEngine


def test_rapid_ocr_reports_missing_optional_dependency(monkeypatch: pytest.MonkeyPatch) -> None:
    original_import = builtins.__import__

    def missing_rapidocr(name: str, *args: object, **kwargs: object) -> object:
        if name == "rapidocr_onnxruntime":
            raise ImportError("missing")
        return original_import(name, *args, **kwargs)

    monkeypatch.setattr(builtins, "__import__", missing_rapidocr)
    engine = RapidOcrEngine()
    with pytest.raises(RuntimeError, match="rapidocr-onnxruntime"):
        engine.read_text(object())
