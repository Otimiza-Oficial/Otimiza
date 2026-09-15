/** Junta classes ignorando valores falsos. Sem dependência: é o que basta aqui. */
export function cn(...classes: Array<string | false | null | undefined>) {
  return classes.filter(Boolean).join(" ");
}
