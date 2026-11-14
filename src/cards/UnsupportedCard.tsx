import { KIND_LABELS } from './registry';
import type { CardProps } from './registry';

/** Card kinds that exist in the data model but have no editor yet. */
export function UnsupportedCard({ card }: CardProps) {
  return (
    <div className="unsupported-card">
      <strong>{KIND_LABELS[card.kind] ?? card.kind}</strong>
      <span>This card type is not supported yet in this version. Its data is kept.</span>
    </div>
  );
}
