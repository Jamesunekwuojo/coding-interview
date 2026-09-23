import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../lib/utils";

const inputVariants = cva(
  "w-full min-w-0 text-body-sm text-foreground transition-colors disabled:cursor-not-allowed disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "rounded-md border border-input bg-card",
        underline: "rounded-none border-b border-input bg-transparent focus-within:border-primary",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

export interface InputProps
  extends React.InputHTMLAttributes<HTMLInputElement>, VariantProps<typeof inputVariants> {
  icon?: React.ReactNode;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type = "text", icon, variant, ...props }, ref) => {
    if (!icon) {
      return (
        <input
          ref={ref}
          type={type}
          className={cn(
            inputVariants({ variant }),
            "placeholder:text-muted-foreground focus-visible:outline-none",
            variant === "underline"
              ? "border-0 focus-visible:border-b px-0 py-2 focus-visible:border-primary"
              : "px-3 py-2 focus-visible:ring-2 focus-visible:ring-ring",
            className,
          )}
          {...props}
        />
      );
    }
    return (
      <div
        className={cn(
          inputVariants({ variant }),
          "flex items-center gap-2",
          variant === "underline" ? "px-0 pb-1" : "px-2",
          className,
        )}
      >
        <span className="flex shrink-0 items-center text-muted-foreground [&>svg]:h-4 [&>svg]:w-4">
          {icon}
        </span>
        <input
          ref={ref}
          type={type}
          className="h-full w-full min-w-0 border-0 bg-transparent p-0 text-inherit placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-0"
          {...props}
        />
      </div>
    );
  },
);

Input.displayName = "Input";
