interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "soft" | "danger";
  size?: "sm" | "md" | "lg";
  children: React.ReactNode;
}

const variants = {
  primary: "bg-gold text-accent-fg hover:bg-accent-hover active:bg-accent-active font-medium",
  secondary:
    "bg-elevated text-content border border-border hover:bg-raised hover:border-border-strong",
  ghost: "bg-transparent text-subtle hover:bg-elevated hover:text-content",
  soft: "bg-accent-soft text-accent-text hover:bg-accent-soft-hover",
  danger: "bg-danger text-white hover:brightness-110 font-medium",
};

const sizes = {
  sm: "h-7 px-2 text-xs rounded-sm",
  md: "h-9 px-3 text-sm rounded-md",
  lg: "h-11 px-5 text-base rounded-lg",
};

export function Button({
  variant = "primary",
  size = "md",
  children,
  className = "",
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      className={`
        inline-flex items-center justify-center gap-1.5 border border-transparent
        tracking-[-0.01em] whitespace-nowrap transition-colors duration-150
        ${sizes[size]}
        ${variants[variant]}
        ${disabled ? "opacity-45 cursor-not-allowed" : "cursor-pointer"}
        ${className}
      `}
      {...props}
    >
      {children}
    </button>
  );
}
