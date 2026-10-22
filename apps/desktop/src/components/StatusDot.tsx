import React from 'react';

export const StatusDot: React.FC<{ status: 'online' | 'offline' | 'pending'; className?: string }> = ({
  status,
  className = '',
}) => <span className={`status-dot ${status} ${className}`} aria-label={`status: ${status}`} />;
