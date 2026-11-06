# AI actions (Arcano edition)

The Generate with AI panel opens on a page of actions instead of a row
of mode buttons. The work is in `apps/tracedraw/src/ui/ai_actions.rs`.

## The page of actions

- A title, "What should the AI do?", and the information button.
- Two groups of tiles, two to a row, each a picture and a name:
  - Make something new: Image from text (Create), Vectors from text
    (Create vectors), Draw with words.
  - On the selected picture: Remove background, Expand, Upscale,
    Variations (Remix), Replace background, Relight, Vectorize. They are
    pale, with a line saying so, while no bitmap is selected.
- A tile opens the action's page: a header with a button back to the
  actions, the action's picture and its name; then the page as before.

## One list for the company and the model

- Create, Remix and Create vectors: one Model list. Under each company
  with a key (its name as a small title) its models, each with what it
  is good at and its price; the main service of Tools > Options > AI at
  the top; the companies without a key at the foot, a click opening the
  keys page. Choosing a model chooses its company too. The list stays
  usable while the chosen service is not ready, to choose another.
  Create vectors lists the vector models of a company that has both.
- The edit actions: one Model list the same way, with each company's
  ways of doing the action and their prices. The row of operation
  buttons is gone: the page of actions chooses the operation.

## From elsewhere

`ai_actions::start` opens the panel at an action. Remove background,
Upscale and Vectorize, which need nothing typed, run at once when a
bitmap is selected and their company has a key.
