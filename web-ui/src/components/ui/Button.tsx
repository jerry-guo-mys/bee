import { cn } from '@/lib/utils';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'outline' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  children: React.ReactNode;
}

export function Button({
  variant = 'primary',
  size = 'md',
  loading = false,
  className,
  children,
  disabled,
  ...props
}: ButtonProps) {
  const baseStyles = `
    inline-flex items-center justify-center gap-2
    font-medium rounded-lg
    transition-all duration-200
    focus:outline-none focus:ring-2 focus:ring-offset-2
    disabled:opacity-50 disabled:cursor-not-allowed
  `;

  const variants = {
    primary: `
      bg-primary-600 text-white
      hover:bg-primary-700
      focus:ring-primary-500
      shadow-sm hover:shadow
    `,
    secondary: `
      bg-surface-200 text-surface-800
      hover:bg-surface-300
      focus:ring-surface-400
      dark:bg-surface-700 dark:text-surface-200
      dark:hover:bg-surface-600
    `,
    outline: `
      border-2 border-surface-300 text-surface-700
      hover:bg-surface-50
      focus:ring-surface-400
      dark:border-surface-600 dark:text-surface-300
      dark:hover:bg-surface-800
    `,
    ghost: `
      text-surface-700
      hover:bg-surface-100
      focus:ring-surface-400
      dark:text-surface-300
      dark:hover:bg-surface-800
    `,
    danger: `
      bg-error-500 text-white
      hover:bg-error-600
      focus:ring-error-500
    `,
  };

  const sizes = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
  };

  return (
    <button
      className={cn(baseStyles, variants[variant], sizes[size], className)}
      disabled={disabled || loading}
      {...props}
    >
      {loading && (
        <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
        </svg>
      )}
      {children}
    </button>
  );
}
