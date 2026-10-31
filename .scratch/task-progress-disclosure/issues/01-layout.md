# Remove crowded sidebar progress card

Status: resolved
Type: improvement

## Progress

- User screenshot shows the multi-line task progress card forcing navigation/footer into sidebar scrolling.
- User subsequently chose the overview's Quick start location. Replace that card with full task progress and an empty state; do not add the initially proposed disclosure.
- Replaced Quick start with the progress card, including the empty state; removed obsolete quick-start strings/style and sidebar card styles. Existing progress events and values are unchanged.
- Type-check/build and focused feedback regression passed. Screenshot checked with task/stage, timing, cycles, role, pause reason and guidance in the overview. Sidebar no longer grows with task detail. Chinese empty state and English populated layout checked at 1000x680.
