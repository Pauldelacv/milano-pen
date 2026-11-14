import { useState } from 'react';
import { useZoom } from '../canvas/cameraController';
import { assetUrl, pickVariant } from '../services/assets';
import type { ImageData } from '../types/models';
import { CaptionEditor } from './CaptionEditor';
import { adjustFilter } from './imageAdjust';
import { ImageToolbar } from './ImageToolbar';
import { useUiStore } from '../stores/uiStore';
import type { CardProps } from './registry';

/** The toolbar only shows when this image is the whole selection. */
function SoloToolbar({ cardId, adjust }: { cardId: string; adjust: ImageData['adjust'] }) {
  const solo = useUiStore((s) => {
    const keys = Object.keys(s.selection);
    return keys.length === 1 && keys[0] === cardId;
  });
  return solo ? <ImageToolbar cardId={cardId} adjust={adjust} /> : null;
}

export function ImageCard({ card, editing, selected }: CardProps) {
  const data = card.data as unknown as ImageData;
  const bucket = useZoom((s) => s.bucket);
  const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
  const variant = pickVariant(card.w * bucket * dpr);
  const [loaded, setLoaded] = useState<Record<string, boolean>>({});
  const [failed, setFailed] = useState(false);
  const ratio = data.width && data.height ? data.height / data.width : card.h ? card.h / card.w : 0.75;
  const filter = adjustFilter(data.adjust);
  return (
    <figure className="image-card">
      {selected && !editing && <SoloToolbar cardId={card.id} adjust={data.adjust} />}
      <div className={`image-frame${loaded['256'] || loaded[variant] ? ' loaded' : ''}`} style={{ aspectRatio: `${1 / ratio}`, filter }}>
        {failed ? (
          <div className="image-missing">Image unavailable</div>
        ) : (
          <>
            {/* The small thumbnail stays underneath while a sharper one loads. */}
            <img
              src={assetUrl(data.assetId, '256')}
              alt={data.caption || data.name || ''}
              decoding="async"
              loading="lazy"
              draggable={false}
              onLoad={() => setLoaded((l) => ({ ...l, 256: true }))}
              onError={() => setFailed(true)}
            />
            {variant !== '256' && (
              <img
                className={loaded[variant] ? 'sharp shown' : 'sharp'}
                src={assetUrl(data.assetId, variant)}
                alt=""
                decoding="async"
                loading="lazy"
                draggable={false}
                onLoad={() => setLoaded((l) => ({ ...l, [variant]: true }))}
              />
            )}
          </>
        )}
      </div>
      {editing ? (
        <CaptionEditor cardId={card.id} value={data.caption ?? ''} placeholder="Add a caption" field="caption" />
      ) : (
        data.caption && <figcaption>{data.caption}</figcaption>
      )}
    </figure>
  );
}
