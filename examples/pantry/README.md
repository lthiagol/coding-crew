# Pantry

Pantry is a recipe app for home cooks. A cook keeps their own recipes,
plans the meals of a week, and turns that week into one shopping list.

## What it is made of

- `pantry-core` — the recipe, the week of meals and the shopping list, as
  plain data.
- `pantry-web` — the small server that serves the app and the public
  recipe pages.
- `data/` — the seed data the app ships with: the demo recipes, the demo
  accounts, a week of meals and a shopping list.

## Running it

    cargo run -p pantry-web          # the app on http://localhost:8080
    cargo test -p pantry-core        # the tests

## How the team works

A change that needs a decision is written down before the code. Reviews
happen before a change reaches the main branch.
