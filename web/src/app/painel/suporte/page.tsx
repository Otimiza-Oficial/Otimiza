import type { Metadata } from "next";
import { Suporte } from "./Suporte";

export const metadata: Metadata = { title: "Suporte" };

export default function Page() {
  return <Suporte />;
}
