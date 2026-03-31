interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "danger";
  children: React.ReactNode;
}

const variants = {
  primary: "bg-gold text-base hover:brightness-110 font-medium",
  secondary: "bg-elevated text-content border border-border hover:bg-surface",
  danger: "bg-danger text-white hover:brightness-110 font-medium",
};

export function Button({
  variant = "primary",
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
        inline-flex items-center justify-center gap-2 rounded-md px-4 h-8 text-sm
        transition-all duration-150
        ${variants[variant]}
        ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}
        ${className}
      `}
      {...props}
    >
      {children}
    </button>
  );
}
