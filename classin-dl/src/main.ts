import { mount } from 'svelte' import App from './App.svelte' const app = mount(App, {
  target: document.body,
  props: {
    // 初始状态
    videoUrl: '',
    outputDir: '',
    loading: false,
    error: null
  }
});

export default app;