"""Geometric journal matching. Input is a whole window, never a user-selected ROI.

Only callers select the eligible map identities (season, difficulty and display).
This module neither captures the desktop nor performs game input. Scores are
distance/coverage heuristics, not probabilities.
"""
from __future__ import annotations

from dataclasses import dataclass
from collections import OrderedDict
from threading import Event
from time import perf_counter
from typing import Callable, Iterable

import cv2
import numpy as np


MAX_PIXELS = 16_777_216
TEMPLATE_SIDE = 800
MIN_REGION_SIDE = 64


class RecognitionCancelled(RuntimeError):
    pass


def _check(cancel: Event | None) -> None:
    if cancel is not None and cancel.is_set():
        raise RecognitionCancelled("Map recognition cancelled")


def _bgr(image: np.ndarray) -> np.ndarray:
    if (image.dtype != np.uint8 or image.ndim not in (2, 3)
            or not image.size or image.shape[0] * image.shape[1] > MAX_PIXELS):
        raise ValueError("Invalid or oversized map image")
    if image.ndim == 2:
        return cv2.cvtColor(image, cv2.COLOR_GRAY2BGR)
    if image.shape[2] == 4:
        # Transparent PNG pixels may contain nonzero, invisible RGB data.
        return ((image[:, :, :3].astype(np.uint16)
                 * image[:, :, 3:4].astype(np.uint16) + 127) // 255).astype(np.uint8)
    if image.shape[2] != 3:
        raise ValueError("Unsupported map image channels")
    return image.copy()


def _features(image: np.ndarray) -> np.ndarray:
    image = _bgr(image)
    hsv = cv2.cvtColor(image, cv2.COLOR_BGR2HSV)
    annotations = ((hsv[:, :, 1] >= 120) & (hsv[:, :, 2] >= 110)).astype(np.uint8)
    annotations = cv2.dilate(annotations, np.ones((5, 5), np.uint8))
    # Ignore the annotation and its boundary rather than inventing structure
    # behind it. This also avoids expensive inpainting of a colorful game scene.
    edges = cv2.Canny(cv2.cvtColor(image, cv2.COLOR_BGR2GRAY), 50, 150)
    edges[annotations != 0] = 0
    return edges


def _map_surface(image: np.ndarray) -> np.ndarray:
    """Solid blue-gray corridors and brown rooms, excluding the map backdrop/HUD."""
    hue, saturation, value = cv2.split(cv2.cvtColor(_bgr(image), cv2.COLOR_BGR2HSV))
    blue = ((hue >= 110) & (hue <= 130) & (saturation >= 15)
            & (saturation <= 130) & (value >= 45) & (value <= 220))
    brown = ((hue >= 3) & (hue <= 30) & (saturation >= 15)
             & (saturation <= 130) & (value >= 45) & (value <= 220))
    # Opening removes thin glyphs and icons without treating their numerous
    # corners as room geometry. The palette is the game's current map palette.
    return cv2.morphologyEx((blue | brown).astype(np.uint8), cv2.MORPH_OPEN,
                           np.ones((5, 5), np.uint8))


@dataclass(frozen=True)
class Template:
    key: str
    edges: np.ndarray
    distance: np.ndarray
    coarse_distance: np.ndarray


def prepare_template(key: str, image: np.ndarray) -> Template:
    image = _bgr(image)
    ratio = TEMPLATE_SIDE / max(image.shape[:2])
    image = cv2.resize(image, (max(1, round(image.shape[1] * ratio)),
                              max(1, round(image.shape[0] * ratio))),
                       interpolation=cv2.INTER_AREA if ratio < 1 else cv2.INTER_LINEAR)
    edges = _features(image)
    if np.count_nonzero(edges) < 100:
        raise ValueError("Map template contains too little visible structure")
    distance = cv2.distanceTransform(255 - edges, cv2.DIST_L2, 3)
    coarse = cv2.resize(distance, (max(1, edges.shape[1] // 4),
                                   max(1, edges.shape[0] // 4)), interpolation=cv2.INTER_AREA) / 4
    return Template(key, edges, distance, coarse)


@dataclass(frozen=True)
class Region:
    x: int
    y: int
    width: int
    height: int

    def crop(self, image: np.ndarray) -> np.ndarray:
        return image[self.y:self.y + self.height, self.x:self.x + self.width]


def locate_regions(frame: np.ndarray, cancel: Event | None = None) -> list[Region]:
    """Propose dark panels and connected wall geometry, then retain original pixels.

    Region proposals are hypotheses. Only subsequent template geometry can accept
    one; neither a fixed corner nor a rectangle alone establishes a map.
    """
    image = _bgr(frame)
    height, width = image.shape[:2]
    ratio = min(1.0, 960 / max(height, width))
    small = cv2.resize(image, (round(width * ratio), round(height * ratio)), interpolation=cv2.INTER_AREA)
    gray = cv2.cvtColor(small, cv2.COLOR_BGR2GRAY)
    edges = cv2.Canny(gray, 40, 120)
    candidates: list[tuple[float, Region]] = []

    def add(x: int, y: int, w: int, h: int, priority: float) -> None:
        x0 = max(0, round((x - 3) / ratio))
        y0 = max(0, round((y - 3) / ratio))
        x1 = min(width, round((x + w + 3) / ratio))
        y1 = min(height, round((y + h + 3) / ratio))
        if min(x1 - x0, y1 - y0) < MIN_REGION_SIDE or not 0.3 <= w / max(h, 1) <= 3.3:
            return
        if np.count_nonzero(edges[max(0, y):y + h, max(0, x):x + w]) < 80:
            return
        candidates.append((priority, Region(x0, y0, x1 - x0, y1 - y0)))

    # Multiple closing radii join broken wall outlines without assuming map size.
    for size in (3, 9, 19):
        _check(cancel)
        joined = cv2.dilate(edges, np.ones((size, size), np.uint8))
        count, _, stats, _ = cv2.connectedComponentsWithStats(joined)
        for x, y, w, h, area in stats[1:count]:
            if area >= 100 and min(w, h) >= 30:
                density = np.count_nonzero(edges[y:y + h, x:x + w]) / (w * h)
                if 0.012 <= density <= 0.4:
                    add(int(x), int(y), int(w), int(h), float(area))
    dark = cv2.morphologyEx((gray < 65).astype(np.uint8), cv2.MORPH_CLOSE, np.ones((9, 9), np.uint8))
    count, _, stats, _ = cv2.connectedComponentsWithStats(dark)
    for x, y, w, h, area in stats[1:count]:
        if min(w, h) >= 40 and area / (w * h) >= 0.65:
            add(int(x), int(y), int(w), int(h), float(area) * 0.5)

    selected: list[Region] = []
    for _, region in sorted(candidates, key=lambda item: item[0], reverse=True):
        duplicate = False
        for previous in selected:
            iw = max(0, min(region.x + region.width, previous.x + previous.width) - max(region.x, previous.x))
            ih = max(0, min(region.y + region.height, previous.y + previous.height) - max(region.y, previous.y))
            union = region.width * region.height + previous.width * previous.height - iw * ih
            if iw * ih / max(union, 1) > 0.85:
                duplicate = True
                break
        if not duplicate:
            selected.append(region)
    # Expanded maps may cover the entire client area. This is scored like every
    # other hypothesis, never accepted simply because other regions failed.
    if np.count_nonzero(edges) >= 100:
        whole = Region(0, 0, width, height)
        if whole not in selected:
            selected.append(whole)
    return selected


@dataclass(frozen=True)
class Match:
    key: str
    score: float
    coverage: float
    unexplained: float
    region: Region
    evidence: float


def _weights(edges: np.ndarray, scale: float) -> np.ndarray:
    return cv2.resize(edges,
                      (max(1, round(edges.shape[1] * scale)), max(1, round(edges.shape[0] * scale))),
                      interpolation=cv2.INTER_AREA if scale < 1 else cv2.INTER_LINEAR)


def _score(template: Template, weights: np.ndarray, explored: np.ndarray,
           x: int, y: int, region: Region, scale: float = 1.0, audit=None, fields=None) -> Match | None:
    metrics = {"scale": float(scale), "x": x, "y": y}

    def outcome(reason):
        if audit is not None:
            audit["attempts"] += 1
            audit["outcomes"][reason] = audit["outcomes"].get(reason, 0) + 1
            cost = metrics.get("geometry_cost", metrics.get("forward_error"))
            if cost is not None and (audit["closest"] is None or cost < audit["closest"]["cost"]):
                audit["closest"] = {**metrics, "cost": cost, "reason": reason}
    height, width = weights.shape
    patch = template.distance[y:y + height, x:x + width]
    mass = float(weights.sum())
    if mass < 100 or patch.shape != weights.shape:
        outcome("insufficient_edges")
        return None
    # Express error in captured-image pixels. A small search scale must not
    # turn an entire corridor network into a tolerably fuzzy single room.
    forward = float((patch * weights).sum() / mass) / scale
    coverage = float(weights[patch <= 3 * scale].sum() / mass)
    metrics.update(forward_error=forward, coverage=coverage)
    if forward > 2.5 or coverage < 0.6:
        outcome("geometry_distance" if forward > 2.5 else "coverage")
        return None
    cached = fields(weights, explored, scale) if fields else None
    trusted = cached[0] if cached else cv2.resize(explored, (width, height), interpolation=cv2.INTER_NEAREST) != 0
    expected = (template.edges[y:y + height, x:x + width] != 0) & trusted
    if np.count_nonzero(expected) < 100:
        outcome("insufficient_template_edges")
        return None
    query_distance = cached[1] if cached else cv2.distanceTransform((weights < 0.2).astype(np.uint8) * 255, cv2.DIST_L2, 3)
    unexplained = float(np.mean(query_distance[expected] > 6 * scale))
    score = forward + 3 * unexplained
    metrics.update(unexplained=unexplained, geometry_cost=score)
    if unexplained > 0.5 or score > 3:
        outcome("unexplained_structure" if unexplained > 0.5 else "geometry_cost")
        return None
    outcome("accepted")
    return Match(template.key, score, coverage, unexplained, region, mass)


def _query(image: np.ndarray, region: Region, audit=None) -> tuple[np.ndarray, np.ndarray, Region] | None:
    def reason(value, **metrics):
        if audit is not None:
            audit.update(reason=value, **metrics)
    surface = _map_surface(image)
    if np.count_nonzero(surface) < max(120, surface.size * 0.04):
        reason("insufficient_map_color", surface_pixels=int(np.count_nonzero(surface)))
        return None
    edges = _features(image)
    explored = cv2.dilate(surface, np.ones((7, 7), np.uint8))
    edges[explored == 0] = 0
    # Avoid treating a panel's outer rectangular rim as map geometry.
    edges[:2] = edges[-2:] = 0
    edges[:, :2] = edges[:, -2:] = 0
    points = cv2.findNonZero(edges)
    if points is None or len(points) < 100:
        reason("insufficient_edges", edge_pixels=0 if points is None else len(points))
        return None
    x, y, width, height = cv2.boundingRect(points)
    # Padding/dilation can make an icon-sized fragment pass the proposal gate.
    # Apply the same minimum to the actual geometry before matching it.
    if min(width, height) < MIN_REGION_SIDE:
        reason("small_visible_region", width=width, height=height)
        return None
    edges = edges[y:y + height, x:x + width]
    explored = explored[y:y + height, x:x + width]
    # A lone HUD panel/button can otherwise match the outline of one room in
    # almost every map. Require more structure than a rectangle before ranking.
    corners = cv2.goodFeaturesToTrack(edges, maxCorners=80, qualityLevel=0.05,
                                     minDistance=max(5, min(width, height) * 0.025))
    if corners is None or len(corners) < 8:
        reason("insufficient_corners", corners=0 if corners is None else len(corners))
        return None
    contours, _ = cv2.findContours(edges, cv2.RETR_LIST, cv2.CHAIN_APPROX_SIMPLE)
    complexity = 0
    for contour in contours:
        _, _, cw, ch = cv2.boundingRect(contour)
        if min(cw, ch) < 16:
            continue
        epsilon = max(1.5, min(4.0, cv2.arcLength(contour, True) * 0.005))
        # Several disconnected room/wall contours are normal in explored maps.
        # Ignore tiny text and the four corners contributed by each rectangle.
        complexity += max(0, len(cv2.approxPolyDP(contour, epsilon, True)) - 4)
    if complexity < 16:
        reason("insufficient_structure", complexity=complexity)
        return None
    visible = Region(region.x + x, region.y + y, width, height)
    reason("accepted", region=vars(visible), corners=len(corners), complexity=complexity)
    # Normalize once per region, not for every scale of every candidate map.
    return edges.astype(np.float32) / 255, explored, visible


def _coarse_weights(edges: np.ndarray, cancel: Event | None = None):
    # These small query images are identical for every map template. Their
    # geometrically increasing areas total about 1 MiB per proposed region.
    samples = []
    for scale in np.exp2(np.arange(-24, 25) / 8):
        _check(cancel)
        h, w = (max(1, round(length * float(scale) / 4)) for length in edges.shape)
        if min(h, w) < 10 or max(h, w) > TEMPLATE_SIDE // 4:
            continue
        weights = _weights(edges, float(scale) / 4)
        mass = float(weights.sum())
        if mass >= 6:
            samples.append((float(scale), weights, mass))
    return samples


def _match_query(template: Template, query: tuple[np.ndarray, np.ndarray, Region],
                 cancel: Event | None = None, coarse_weights=None, resize=_weights, audit=None, fields=None) -> Match | None:
    edges, explored, visible = query
    coarse = template.coarse_distance
    seeds: list[tuple[float, float, int, int]] = []
    for scale, weights, mass in (_coarse_weights(edges, cancel) if coarse_weights is None else coarse_weights):
        _check(cancel)
        if weights.shape[0] > coarse.shape[0] or weights.shape[1] > coarse.shape[1]:
            continue
        costs = cv2.matchTemplate(coarse, weights, cv2.TM_CCORR) / mass
        value, _, location, _ = cv2.minMaxLoc(costs)
        seeds.append((value, float(scale), location[0] * 4, location[1] * 4))
    best: Match | None = None
    for _, seed_scale, seed_x, seed_y in sorted(seeds)[:4]:
        for scale in seed_scale * np.exp2(np.linspace(-0.125, 0.125, 9)):
            _check(cancel)
            h, w = (max(1, round(length * float(scale))) for length in edges.shape)
            th, tw = template.distance.shape
            if h > th or w > tw:
                continue
            weights = resize(edges, float(scale))
            left, top = max(0, seed_x - 12), max(0, seed_y - 12)
            right, bottom = min(tw, seed_x + w + 12), min(th, seed_y + h + 12)
            area = template.distance[top:bottom, left:right]
            if area.shape[0] < h or area.shape[1] < w:
                continue
            costs = cv2.matchTemplate(area, weights, cv2.TM_CCORR)
            _, _, location, _ = cv2.minMaxLoc(costs)
            found = _score(template, weights, explored, left + location[0], top + location[1], visible, float(scale), audit, fields)
            if found is not None and (best is None or found.score < best.score):
                best = found
    return best


def match_region(template: Template, image: np.ndarray, region: Region,
                 cancel: Event | None = None) -> Match | None:
    _check(cancel)
    query = _query(image, region)
    return _match_query(template, query, cancel) if query is not None else None


def recognize(frame: np.ndarray, templates: Iterable[Template], cancel: Event | None = None,
              progress: Callable[[str, int, int], None] | None = None,
              template_count: int = 0, diagnostic=None, image=None, preview=None) -> list[Match]:
    _check(cancel)
    if progress:
        progress("locating", 0, 0)
    regions = locate_regions(frame, cancel)
    whole = Region(0, 0, frame.shape[1], frame.shape[0])
    include_whole = whole in regions
    regions = [region for region in regions if region != whole]
    queries = []
    def inspect(region):
        audit = {"proposal": vars(region)}
        started = perf_counter()
        query = _query(region.crop(frame), region, audit)
        if diagnostic:
            diagnostic({"stage": "region_check", **audit, "elapsed_ms": round((perf_counter() - started) * 1000)})
        if query is not None and image:
            index = len(queries)
            image(f"region_{index}", query[2].crop(frame))
            image(f"edges_{index}", (query[0] * 255).astype(np.uint8))
            image(f"mask_{index}", query[1] * 255)
        return query
    for region in regions:
        _check(cancel)
        query = inspect(region)
        if query is not None:
            queries.append(query)
        # Discard invalid HUD regions before applying the matching work bound.
        if len(queries) == 12:
            break
    if include_whole:
        _check(cancel)
        query = inspect(whole)
        if query is not None:
            queries.append(query)
    if not queries:
        return []
    if preview:
        preview(queries[0][2])
    samples = [_coarse_weights(query[0], cancel) for query in queries]
    # Many templates select the same refinement scales. Cache by actual output
    # dimensions/interpolation, with a byte bound rather than a map-count bound.
    # The cache belongs to this call and is released on completion/cancellation.
    resized = OrderedDict()
    resized_bytes = 0

    def cached(key, create):
        nonlocal resized_bytes
        values = resized.pop(key, None)
        if values is None:
            values = create()
            resized_bytes += sum(value.nbytes for value in values)
        resized[key] = values
        while resized_bytes > 32 * 1024 * 1024:
            resized_bytes -= sum(value.nbytes for value in resized.popitem(last=False)[1])
        return values

    def resize(edges, scale):
        width, height = (max(1, round(length * scale)) for length in edges.shape[::-1])
        return cached(("weights", id(edges), width, height, scale < 1), lambda: (_weights(edges, scale),))[0]

    def fields(weights, explored, scale):
        height, width = weights.shape
        return cached(("fields", id(explored), width, height, scale < 1), lambda: (
            cv2.resize(explored, (width, height), interpolation=cv2.INTER_NEAREST) != 0,
            cv2.distanceTransform((weights < 0.2).astype(np.uint8) * 255, cv2.DIST_L2, 3)))
    # Templates may be streamed from disk so a large library does not retain
    # every decoded image and distance field at once.
    found: dict[str, Match] = {}
    completed = 0
    shown_region = queries[0][2]
    for index, template in enumerate(templates):
        _check(cancel)
        if progress:
            progress("matching", index, template_count)
        for region_index, (query, coarse_weights) in enumerate(zip(queries, samples)):
            audit = {"attempts": 0, "outcomes": {}, "closest": None}
            started = perf_counter()
            match = _match_query(template, query, cancel, coarse_weights, resize, audit, fields)
            if diagnostic:
                diagnostic({"stage": "template_region", "key": template.key, "region_index": region_index,
                            "region": vars(query[2]), **audit,
                            "elapsed_ms": round((perf_counter() - started) * 1000),
                            "accepted": match is not None,
                            "match": None if match is None else {"geometry_cost": match.score,
                                "coverage": match.coverage, "unexplained": match.unexplained}})
            if match is not None:
                previous = found.get(match.key)
                # When both minimap and expanded map match, prefer substantially
                # more normalized wall evidence; use score for comparable views.
                if previous is None or match.evidence > previous.evidence * 1.25 or (
                        match.evidence >= previous.evidence / 1.25 and match.score < previous.score):
                    found[match.key] = match
        if found and preview:
            leading = min(found.values(), key=lambda match: (match.score, match.key))
            if leading.region != shown_region:
                shown_region = leading.region
                preview(shown_region)
        completed = index + 1
    _check(cancel)
    if progress:
        progress("matching", completed, template_count or completed)
    # Multiple genuinely plausible maps are intentional; no forced top-one or
    # arbitrary top-N truncation. Caller must always require the user's click.
    return sorted(found.values(), key=lambda match: (match.score, match.key))
