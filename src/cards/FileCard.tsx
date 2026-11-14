import { File, FileArchive, FileAudio, FileImage, FileSpreadsheet, FileText, FileVideo, Presentation } from 'lucide-react';
import { formatBytes } from '../services/assets';
import type { FileData } from '../types/models';
import type { CardProps } from './registry';

function iconFor(mime = '', name = '') {
  const ext = name.split('.').pop()?.toLowerCase() ?? '';
  if (mime.startsWith('image/')) return FileImage;
  if (mime.startsWith('audio/')) return FileAudio;
  if (mime.startsWith('video/')) return FileVideo;
  if (mime === 'application/pdf' || mime.startsWith('text/') || ['doc', 'docx', 'pages', 'rtf'].includes(ext)) return FileText;
  if (['xls', 'xlsx', 'numbers', 'csv'].includes(ext)) return FileSpreadsheet;
  if (['ppt', 'pptx', 'key'].includes(ext)) return Presentation;
  if (['zip', 'gz', 'tar', '7z', 'rar'].includes(ext)) return FileArchive;
  return File;
}

export function FileCard({ card }: CardProps) {
  const data = card.data as unknown as FileData;
  const Icon = iconFor(data.mime, data.name);
  const ext = data.name?.split('.').pop()?.toUpperCase();
  return (
    <div className="file-card" title="Double-click to open">
      <div className="file-icon">
        <Icon size={22} strokeWidth={1.6} />
      </div>
      <div className="file-meta">
        <div className="file-name">{data.name || 'File'}</div>
        <div className="file-sub">
          {ext && ext.length <= 5 ? `${ext} · ` : ''}
          {formatBytes(data.size)}
        </div>
      </div>
    </div>
  );
}
