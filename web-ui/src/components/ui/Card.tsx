import { cn } from '@/lib/utils';

export interface CardProps {
  className?: string;
  children: React.ReactNode;
}

export function Card({ className, children }: CardProps) {
  return (
    <div
      className={cn(
        'bg-white dark:bg-surface-800',
        'rounded-xl shadow-card',
        'border border-surface-200 dark:border-surface-700',
        className
      )}
    >
      {children}
    </div>
  );
}

export function CardHeader({ className, children }: CardProps) {
  return (
    <div className={cn('px-6 py-4 border-b border-surface-200 dark:border-surface-700', className)}>
      {children}
    </div>
  );
}

export function CardTitle({ className, children }: CardProps) {
  return (
    <h3 className={cn('text-lg font-semibold text-surface-900 dark:text-surface-100', className)}>
      {children}
    </h3>
  );
}

export function CardDescription({ className, children }: CardProps) {
  return (
    <p className={cn('text-sm text-surface-500 dark:text-surface-400 mt-1', className)}>
      {children}
    </p>
  );
}

export function CardContent({ className, children }: CardProps) {
  return (
    <div className={cn('px-6 py-4', className)}>
      {children}
    </div>
  );
}

export function CardFooter({ className, children }: CardProps) {
  return (
    <div className={cn('px-6 py-4 border-t border-surface-200 dark:border-surface-700', className)}>
      {children}
    </div>
  );
}
