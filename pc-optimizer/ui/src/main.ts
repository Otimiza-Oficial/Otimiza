import { mount } from "svelte";
import "./tokens.css";
import App from "./App.svelte";

const alvo = document.getElementById("app");
if (!alvo) throw new Error("index.html sem #app");

export default mount(App, { target: alvo });
