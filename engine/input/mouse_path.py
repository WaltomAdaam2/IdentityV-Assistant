from __future__ import annotations

import random
from math import ceil, hypot


def curved_route(
    start: tuple[int, int],
    end: tuple[int, int],
    rng: random.Random | None = None,
) -> list[tuple[int, int]]:
    randomizer = rng or random
    dx, dy = end[0] - start[0], end[1] - start[1]
    distance = hypot(dx, dy)
    if distance < 4:
        return [end]

    nx, ny = -dy / distance, dx / distance
    bend = randomizer.choice((-1, 1)) * min(240, distance * randomizer.uniform(0.25, 0.55))
    # Some routes begin away from the target before turning toward it.
    first = randomizer.uniform(-0.30, -0.12) if randomizer.random() < 0.4 else randomizer.uniform(0.15, 0.4)
    second = randomizer.uniform(0.65, 1.18)
    other_bend = bend * randomizer.uniform(-0.8, 0.8)
    c1 = (start[0] + dx * first + nx * bend, start[1] + dy * first + ny * bend)
    c2 = (start[0] + dx * second + nx * other_bend, start[1] + dy * second + ny * other_bend)
    polygon_length = hypot(c1[0] - start[0], c1[1] - start[1]) + hypot(c2[0] - c1[0], c2[1] - c1[1]) + hypot(end[0] - c2[0], end[1] - c2[1])
    steps = max(24, ceil(polygon_length / 12))
    route: list[tuple[int, int]] = []
    for index in range(1, steps + 1):
        progress = index / steps
        t = progress * progress * (3 - 2 * progress)
        inverse = 1 - t
        candidate = tuple(round(inverse**3 * start[axis] + 3 * inverse**2 * t * c1[axis] + 3 * inverse * t**2 * c2[axis] + t**3 * end[axis]) for axis in (0, 1))
        if candidate != (route[-1] if route else start):
            route.append(candidate)
    if not route or route[-1] != end:
        route.append(end)
    return route


def route_duration(start: tuple[int, int], route: list[tuple[int, int]]) -> float:
    length = sum(hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip([start, *route[:-1]], route))
    return min(0.85, max(0.18, length / 2400))
