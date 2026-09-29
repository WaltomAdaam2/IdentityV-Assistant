from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any

from core.models import Rect


@dataclass(frozen=True)
class OcrResult:
    text: str
    confidence: float
    bbox: Rect


class OcrEngine(ABC):
    @abstractmethod
    def read_text(self, image: Any) -> list[OcrResult]: ...


class RapidOcrEngine(OcrEngine):
    def __init__(self) -> None:
        self._engine: Any | None = None

    def read_text(self, image: Any) -> list[OcrResult]:
        if self._engine is None:
            try:
                from rapidocr_onnxruntime import RapidOCR
            except ImportError as error:
                raise RuntimeError(
                    "RapidOCR support requires the optional 'rapidocr-onnxruntime' dependency. "
                    "Install it with: pip install identityv-assistant-engine[ocr]"
                ) from error
            self._engine = RapidOCR()

        detected, _ = self._engine(image)
        if detected is None:
            return []
        results: list[OcrResult] = []
        for bbox, text, confidence in detected:
            xs = [round(float(point[0])) for point in bbox]
            ys = [round(float(point[1])) for point in bbox]
            left, top = min(xs), min(ys)
            results.append(OcrResult(str(text), float(confidence), Rect(left, top, max(xs) - left, max(ys) - top)))
        return results
