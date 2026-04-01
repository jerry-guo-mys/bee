import { cn } from '@/lib/utils';

export function Badge({
  children,
  variant = 'default',
  className,
}: {
  children: React.ReactNode;
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
  className?: string;
}) {
  const variants = {
    default: 'bg-surface-100 text-surface-700 dark:bg-surface-800 dark:text-surface-300',
    success: 'bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-400',
    warning: 'bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-400',
    error: 'bg-error-100 text-error-700 dark:bg-error-900/30 dark:text-error-400',
    info: 'bg-primary-100 text-primary-700 dark:bg-primary-900/30 dark:text-primary-400',
  };

  return (
    <span
      className={cn(
        'inline-flex items-center px-2.5 py-0.5',
        'rounded-full text-xs font-medium',
        variants[variant],
        className
      )}
    >
      {children}
    </span>
  );
}
