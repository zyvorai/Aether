import { forwardRef, type ButtonHTMLAttributes } from 'react';
import { cn } from '../../lib/cn';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'link' | 'tertiary';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: 'sm' | 'md' | 'lg';
}

const variants: Record<Variant, string> = {
  primary:
    'rounded-[var(--radius-pill)] text-white bg-primary border border-primary ' +
    'hover:bg-[var(--primary-hover)] hover:-translate-y-px hover:shadow-[var(--shadow-accent)] ' +
    'active:translate-y-0 active:scale-[0.98] transition-all',
  secondary:
    'rounded-[var(--radius-pill)] text-primary border border-primary bg-transparent ' +
    'hover:bg-[var(--accent-tint)] hover:-translate-y-px active:scale-[0.98] transition-all',
  ghost:
    'rounded-[var(--radius-sm)] text-muted hover:text-foreground hover:bg-[var(--nav-hover-bg)] transition-colors',
  danger:
    'rounded-[var(--radius-pill)] bg-danger/10 text-danger hover:bg-danger/15 transition-colors',
  link: 'text-primary hover:underline p-0 h-auto font-normal',
  tertiary:
    'rounded-[var(--radius-sm)] text-primary bg-transparent px-3 py-2 ' +
    'hover:bg-[var(--accent-tint)] active:scale-[0.98] transition-all',
};

const sizes = {
  sm: 'px-3.5 py-1.5 text-body-sm min-h-[36px]',
  md: 'px-5 py-2 text-body min-h-[44px]',
  lg: 'px-6 py-2.5 text-body font-medium min-h-[44px]',
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'primary', size = 'md', disabled, ...props }, ref) => (
    <button
      ref={ref}
      disabled={disabled}
      className={cn(
        'inline-flex items-center justify-center font-medium focus-ring disabled:opacity-50 disabled:pointer-events-none',
        variant !== 'link' && sizes[size],
        variants[variant],
        className,
      )}
      {...props}
    />
  ),
);
Button.displayName = 'Button';
