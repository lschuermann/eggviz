const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const dist = path.resolve(__dirname, "dist");

module.exports = {
    mode: "production",
    entry: {
        index: "./js/index.js"
    },
    module: {
        rules: [{
                test: /\.(wasm)$/,
                type: "webassembly/async"
            },
            {
                test: /\.css$/i,
                use: ["style-loader", "css-loader"],
            },
        ],
    },
    resolve: {
        extensions: ['.js', '.wasm'],
    },

    output: {
        path: dist,
        publicPath: "",
        filename: "[name].js"
    },
    plugins: [
        new CopyPlugin({
            patterns: [path.resolve(__dirname, "static")],
        }),

        new WasmPackPlugin({
            crateDirectory: __dirname,
        }),
    ],
    experiments: {
        asyncWebAssembly: true,
    },
    performance: {
        hints: false,
        maxEntrypointSize: 1024 * 1024,
        maxAssetSize: 1024 * 1024,
    },
};