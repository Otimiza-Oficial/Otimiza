import type { Metadata } from "next";
import { Licencas } from "./Licencas";

export const metadata: Metadata = { title: "Licenças" };

export default function Page() {
  return <Licencas />;
}
