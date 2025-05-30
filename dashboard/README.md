# Run and deploy the Taikoscope dashboard

This contains everything you need to run your app locally.

## Run Locally

**Prerequisites:**

- Node.js

1. Install dependencies:
   `npm install`
2. Run the app:
   `npm run dev`
3. Build for production:
   `npm run build`

The production build bundles React and other dependencies locally and uses self-hosted fonts to avoid external network requests. The font files are not included in the repository. To self-host fonts, copy the required `.ttf` files into `dashboard/public/fonts` before building.
