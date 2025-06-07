# Run and deploy the Taikoscope dashboard

This contains everything you need to run your app locally.

## Run Locally

**Prerequisites:**

- Node.js

1. Install dependencies:
   `npm install`
2. Set up fonts (if not already present):
   `./setup-fonts.sh`
3. Run the app:
   `npm run dev`
4. Run with mocked data (no API required):
   `npm run dev:mock`
5. Build for production:
   `npm run build`
6. Check for trailing whitespace:
   `npm run lint:whitespace`
