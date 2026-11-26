import { forwardRef, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { Icon } from "../../icons/Icon";

export interface FieldProps {
  label?: string;
  hint?: string;
  error?: string;
  htmlFor?: string;
  children: ReactNode;
}

export function Field({ label, hint, error, htmlFor, children }: FieldProps) {
  return (
    <label className="sory-field" htmlFor={htmlFor}>
      {label ? <span className="sory-field-label">{label}</span> : null}
      {children}
      {error ? (
        <span className="sory-field-hint sory-field-hint-error">
          {error}
        </span>
      ) : hint ? (
        <span className="sory-field-hint">{hint}</span>
      ) : null}
    </label>
  );
}

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { className, ...props },
  ref,
) {
  return (
    <input ref={ref} className={["sory-input", className].filter(Boolean).join(" ")} {...props} />
  );
});

export interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(function Textarea(
  { className, ...props },
  ref,
) {
  return (
    <textarea
      ref={ref}
      className={["sory-textarea", className].filter(Boolean).join(" ")}
      {...props}
    />
  );
});

export interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(function Select(
  { className, children, ...props },
  ref,
) {
  return (
    <select
      ref={ref}
      className={["sory-select", className].filter(Boolean).join(" ")}
      {...props}
    >
      {children}
    </select>
  );
});

export interface SearchInputProps extends InputHTMLAttributes<HTMLInputElement> {}

export const SearchInput = forwardRef<HTMLInputElement, SearchInputProps>(function SearchInput(
  { className, ...props },
  ref,
) {
  return (
    <span className="sory-search">
      <span className="sory-search-icon">
        <Icon name="search" size={14} />
      </span>
      <input
        ref={ref}
        type="search"
        className={["sory-input", className].filter(Boolean).join(" ")}
        {...props}
      />
    </span>
  );
});
