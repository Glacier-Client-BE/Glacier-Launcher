import { mount } from 'svelte'
import './app.css'
import './interop.js'
import App from './App.svelte'

const app = mount(App, {
  target: document.getElementById('root')!,
})

export default app
