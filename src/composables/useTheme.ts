import {Window, Theme} from '@tauri-apps/api/window';
import {onBeforeMount, onBeforeUnmount, readonly, ref} from "vue";


export const useTheme = () => {
    const theme = ref<Theme>('light');
    let unlisten: () => void
    const appWindow = Window.getCurrent();

    onBeforeMount(async () => {
        theme.value = await appWindow.theme() ?? theme.value;

        unlisten = await appWindow.onThemeChanged(({payload}) => {
            console.log(`theme changed to ${payload}`);
            theme.value = payload
        })
    })
    onBeforeUnmount(() => unlisten?.());

    return {
        theme: readonly(theme),
    }
};
