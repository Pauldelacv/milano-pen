import { ExternalLink, Link2 } from 'lucide-react';
import { openLink } from '../app/actions';
import type { LinkData } from '../types/models';
import { CaptionEditor } from './CaptionEditor';
import { embedUrl, hostOf } from './embeds';
import type { CardProps } from './registry';

export function LinkCard({ card, editing, selected }: CardProps) {
  const data = card.data as unknown as LinkData;
  const embed = embedUrl(data.url);
  return (
    <div className="link-card">
      {embed && (
        <div className={`embed${selected ? ' live' : ''}`}>
          <iframe
            src={embed}
            title={data.title || hostOf(data.url)}
            loading="lazy"
            allow="encrypted-media; picture-in-picture; fullscreen"
            referrerPolicy="strict-origin-when-cross-origin"
            sandbox="allow-scripts allow-same-origin allow-presentation allow-popups"
          />
        </div>
      )}
      <div className="link-body">
        <Link2 size={14} className="link-icon" />
        <div className="link-text">
          {editing ? (
            <CaptionEditor cardId={card.id} value={data.title ?? ''} placeholder="Link title" field="title" label="Edit link" />
          ) : (
            <div className="link-title">{data.title || hostOf(data.url)}</div>
          )}
          <div className="link-url">{hostOf(data.url)}</div>
        </div>
        <button className="icon-button" data-no-drag title="Open in browser" onClick={() => void openLink(data.url)}>
          <ExternalLink size={14} />
        </button>
      </div>
    </div>
  );
}
