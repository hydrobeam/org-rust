import path from 'path';
import * as url from 'url';
const __dirname = url.fileURLToPath(new URL('.', import.meta.url));

import HtmlBundlerPlugin from "html-bundler-webpack-plugin";
import TerserPlugin from 'terser-webpack-plugin';
import CssMinimizerPlugin from 'css-minimizer-webpack-plugin';
import HtmlMinimizerPlugin from 'html-minimizer-webpack-plugin';



export default {
  mode: 'production',
  output: {
    path: path.resolve(__dirname, 'dist'),
    clean: true,
  },
  devServer: {
    watchFiles: ["./*"],
    client: {
      overlay: {
        errors: true,
        warnings: false,
        runtimeErrors: true,
      }
    },
  },
  plugins: [
    // allows us to set html as the main entrypoint
    new HtmlBundlerPlugin({
      entry: {
        index: "./index.html",
        iframe: "./iframe.html",
      },
    })
  ],
  experiments: {
    asyncWebAssembly: true,
  },
  module: {
    rules: [
      {
        test: /\.org$/,
        type: 'asset/source'
      },
      // styles
      {
        test: /\.css$/,
        use: ['css-loader'],
      },
      {
        test: /\.tsx?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ]
  },
  optimization: {
    minimizer: [
      new HtmlMinimizerPlugin(
        {
          minify: HtmlMinimizerPlugin.minifyHtmlNode
        }
      ),
      // mirroring the default webpack config
      // adding the other minimizers causes this one to not activate,
      // so be explicit about using it
      new TerserPlugin({
        terserOptions: {
          compress: {
            passes: 2
          }
        }
      }),
      new CssMinimizerPlugin()
    ]
  }
};
