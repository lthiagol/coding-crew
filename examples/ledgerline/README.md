# Ledgerline

Ledgerline is a command-line tool for the payments team. It reads a batch of
payments, matches each one against the ledger, and writes a reconciliation
report the finance team can keep.

## The commands

    ledgerline import batches.csv     # read a batch into the ledger
    ledgerline reconcile              # match the batch and report
    ledgerline export --double-entry  # the ledger as double-entry lines

## What it is made of

- `ledgerline-core` — the batch, the ledger and the matcher.
- `fixtures/` — the sample files the tool ships with: a batch of payments
  and what the gateway said about them.

## Running it

    cargo run -p ledgerline -- reconcile
    cargo test -p ledgerline-core

## How the team works

Every change that touches the money path is reviewed by someone from
finance. A run that is interrupted leaves its partial batch on disk.
The report is coloured when it is going to a terminal that can show colour.
