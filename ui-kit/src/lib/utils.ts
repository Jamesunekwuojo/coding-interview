import { clsx, type ClassValue } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

const isTypeScaleRole = (value: string) =>
  /^(caption|label|body-[a-z]+|heading-[1-6]|display-[1-2])$/.test(value);

const twMerge = extendTailwindMerge({
  extend: { classGroups: { "font-size": [{ text: [isTypeScaleRole] }] } },
});

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
