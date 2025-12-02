declare module 'vue3-infinite-list' {
  import { DefineComponent, Slot } from 'vue'

  export type ItemSizeGetter = (index: number) => number
  export type ItemSize = number | number[] | ItemSizeGetter

  export interface ILEvent {
    items: any[]
    offset: number
    data?: any[] | null
    start?: number
    stop?: number
    total?: number
    toString(): string
  }

  export interface ItemStyle {
    position: 'absolute'
    top?: number
    left: number
    width: string | number
    height?: number
  }

  export interface StyleCache {
    [id: number]: ItemStyle
  }

  export interface ItemInfo {
    index: number
    style: ItemStyle
  }

  export interface RenderedRows {
    startIndex: number
    stopIndex: number
  }

  export interface InfiniteListProps {
    /**
     * スクロール方向
     * @default 'vertical'
     */
    scrollDirection?: 'vertical' | 'horizontal'

    /**
     * スクロール時のアライメント
     * @default 'auto'
     */
    scrollToAlignment?: 'auto' | 'start' | 'center' | 'end'

    /**
     * 表示領域外にレンダリングするアイテム数
     * @default 4
     */
    overscanCount?: number

    /**
     * アイテムのサイズ（固定値、配列、または関数）
     */
    itemSize: ItemSize

    /**
     * 表示するデータ配列
     */
    data: any[] | null

    /**
     * サイズの単位
     * @default 'px'
     */
    unit?: string

    /**
     * リストの幅
     */
    width?: number | string

    /**
     * リストの高さ
     */
    height?: number | string

    /**
     * デバッグモード
     * @default false
     */
    debug?: boolean

    /**
     * スクロールオフセット
     */
    scrollOffset?: number

    /**
     * スクロール先のインデックス
     */
    scrollToIndex?: number

    /**
     * 推定アイテムサイズ
     */
    estimatedItemSize?: number
  }

  export interface InfiniteListSlots {
    default?(props: {
      event: ILEvent
      item: any
      index: number
    }): any
  }

  const InfiniteList: DefineComponent<InfiniteListProps>

  export default InfiniteList
}
