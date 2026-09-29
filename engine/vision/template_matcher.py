from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class TemplateMatch:
    found: bool
    confidence: float
    bbox: tuple[int, int, int, int] | None


class TemplateMatcher:
    def match(self, image: Any, template: Any, threshold: float = 0.85) -> TemplateMatch:
        import cv2

        if image is None or template is None:
            raise ValueError("image and template are required")
        image_height, image_width = image.shape[:2]
        template_height, template_width = template.shape[:2]
        if template_height > image_height or template_width > image_width:
            return TemplateMatch(False, 0.0, None)
        result = cv2.matchTemplate(image, template, cv2.TM_CCOEFF_NORMED)
        _, maximum, _, location = cv2.minMaxLoc(result)
        bbox = (int(location[0]), int(location[1]), template_width, template_height)
        return TemplateMatch(maximum >= threshold, float(maximum), bbox)
