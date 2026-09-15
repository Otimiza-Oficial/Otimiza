import type { Metadata } from "next";
import { Downloads } from "./Downloads";

export const metadata: Metadata = { title: "Downloads" };

export default function Page() {
  return <Downloads />;
}
