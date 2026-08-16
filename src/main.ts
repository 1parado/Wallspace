import { createApp } from 'vue';
import { createPinia } from 'pinia';
import '@fontsource-variable/inter';
import './styles/tokens.css';
import './styles/base.css';
import App from './App.vue';

createApp(App).use(createPinia()).mount('#app');
