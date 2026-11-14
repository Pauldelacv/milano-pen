-- Milano Pen schema v1.
-- Model adapted from Milaclone (MIT, (c) 2026 ellshep): boards form a tree,
-- cards belong to a board and may sit inside a column card, `data` is per-kind
-- JSON, and deletes only tag rows (`trashed_by`) so they can be restored.

CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;

CREATE TABLE boards (
  id              TEXT PRIMARY KEY,
  parent_board_id TEXT,
  title           TEXT NOT NULL DEFAULT 'Untitled board',
  color           TEXT,
  icon            TEXT,
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL,
  trashed_by      TEXT,
  trashed_at      INTEGER
) STRICT;

CREATE TABLE cards (
  id             TEXT PRIMARY KEY,
  board_id       TEXT NOT NULL,
  parent_card_id TEXT,
  kind           TEXT NOT NULL,
  x              REAL NOT NULL,
  y              REAL NOT NULL,
  w              REAL NOT NULL,
  h              REAL,
  z              INTEGER NOT NULL DEFAULT 0,
  color          TEXT,
  data           TEXT NOT NULL DEFAULT '{}',
  search_text    TEXT NOT NULL DEFAULT '',
  created_at     INTEGER NOT NULL,
  updated_at     INTEGER NOT NULL,
  trashed_by     TEXT,
  trashed_at     INTEGER
) STRICT;

CREATE TABLE assets (
  id            TEXT PRIMARY KEY,       -- sha256 of the content, hex
  ext           TEXT NOT NULL,
  mime          TEXT NOT NULL,
  size          INTEGER NOT NULL,
  width         INTEGER,
  height        INTEGER,
  original_name TEXT NOT NULL,
  created_at    INTEGER NOT NULL
) STRICT;

-- Hot paths: the live cards of one board, column contents, trash roots.
CREATE INDEX idx_cards_board_live ON cards(board_id, z) WHERE trashed_by IS NULL;
CREATE INDEX idx_cards_parent ON cards(parent_card_id) WHERE parent_card_id IS NOT NULL;
CREATE INDEX idx_cards_trashed_by ON cards(trashed_by) WHERE trashed_by IS NOT NULL;
CREATE INDEX idx_cards_asset ON cards(json_extract(data, '$.assetId'))
  WHERE json_extract(data, '$.assetId') IS NOT NULL;
CREATE INDEX idx_boards_parent ON boards(parent_board_id);
CREATE INDEX idx_boards_trashed_by ON boards(trashed_by) WHERE trashed_by IS NOT NULL;
