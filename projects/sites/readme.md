# sites

本目录 **不在** pnpm workspace 内。homepage / playground 只依赖 **已发布** 的 `@game-gpt/red-alert2@0.0.0`，禁止
`workspace:*`。

```shell
# 先在仓库根发布占位包，再：
cd projects/sites/homepage
npm i
npm run dev
```
