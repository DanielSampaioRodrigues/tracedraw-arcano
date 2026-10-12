# Shaping

Object > Shaping, the property bar's shaping buttons (with two or more
objects selected) and the Shaping panel.

## Operations

The last object selected is the target and gives the result its fill,
outline and opacity; the other selected objects are the sources.

| Operation | Needs | Result | What stays (menu, property bar) |
|---|---|---|---|
| Weld | 2 | one object covering all of them | nothing |
| Trim | 2 | the target without what the sources cover | the sources |
| Intersect | 2 | the area all of them share | the sources and the target |
| Simplify | 1 | each object without what objects above it cover | the objects, changed in place |
| Front Minus Back | 2 | the front object without the ones behind (its attributes) | nothing |
| Back Minus Front | 2 | the back object without the ones in front (its attributes) | nothing |
| Boundary | 1 | an outline around the selection (the target's attributes) | every object |

The result goes on top of the active layer and becomes the selection; an
empty result changes nothing and the status bar says so. Each operation
is one undo step.

## Panel

- Seven picture buttons choose the operation; under them its name, a
  large picture and what it does, and that the last object selected is
  the target.
- Weld, Trim and Intersect each keep their own choice of what to leave:
  "Leave original source object" and "Leave original target object"
  (Weld: neither, Trim: the sources, Intersect: both, as the menus do).
  The choices and the operation are kept in the preferences
  (`panels.shaping_op`, `panels.shaping_leave`).
- The button under them, named after the operation, applies it; it is
  pale until enough objects are selected, and a line says how many.

## Checks

- `ui::shaping_docker::tests::what_the_operations_leave_of_the_originals`
- `ui::shaping_docker::tests::the_docker_draws_every_operation`
- `ui::propbar::tests::shaping_and_grouping_show_for_several_objects_groups_and_parts`
