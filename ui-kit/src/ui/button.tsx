import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../lib/utils";

export const buttonVariants = cva(
  "inline-flex items-center text-body-sm font-semibold py-2 px-4 no-underline whitespace-nowrap cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-not-allowed justify-center items-center",
  {
    variants: {
      variant: {
        primary: "bg-primary text-primary-foreground hover:bg-primary-hover",
        outline: "border border-input bg-card text-foreground hover:bg-accent",
        primaryInverse:
          "bg-primary-foreground text-primary border-primary-foreground hover:bg-info-muted",
        ghost: "bg-transparent text-foreground hover:bg-accent",
        danger: "bg-transparent text-destructive border-destructive/30 hover:bg-destructive/10",
      },
      size: {
        default: "",
        sm: "text-body-sm py-2 px-3",
        lg: "text-base py-3 px-6 gap-2",
      },
      shape: {
        default: "rounded-md",
        circle: "rounded-full",
        square: "rounded-none",
      },
    },
    defaultVariants: { variant: "primary", size: "default", shape: "default" },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, shape, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        ref={ref}
        className={cn(buttonVariants({ variant, size, shape }), className)}
        {...props}
      />
    );
  },
);

Button.displayName = "Button";
