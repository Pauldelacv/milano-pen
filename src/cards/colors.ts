/** Card and board colours (Milaclone's palette). CSS: --c-<name>, --card-<name>. */
export const COLORS = ['slate', 'gray', 'teal', 'green', 'brown', 'yellow', 'orange', 'red', 'pink', 'purple', 'blue', 'indigo'] as const;
export type ColorName = (typeof COLORS)[number];
