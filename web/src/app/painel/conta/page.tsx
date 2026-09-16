import type { Metadata } from "next";
import { Conta } from "./Conta";

export const metadata: Metadata = { title: "Preferências" };

export default function Page() {
  return <Conta />;
}
