import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

// Joins class names and lets later Tailwind classes win (the shadcn/ui helper).
export function cn(...classes: ClassValue[]): string {
  return twMerge(clsx(classes));
}
