import { mount } from 'svelte'
import GameConsoleApp from './GameConsoleApp.svelte'

const app = mount(GameConsoleApp, {
  target: document.getElementById('root')!,
})

export default app
