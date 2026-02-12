/** @type {import('tailwindcss').Config} */
export default {
    content: ['./src/**/*.{html,js,svelte,ts}'],
    theme: {
        extend: {
            colors: {
                'bg-grain': '#FAF9F6',
                'bg-subtle': '#F3F4F6',
                'text-main': '#374151',
                'text-sub': '#9CA3AF',
                'text-light': '#D1D5DB',
                'accent-blue': '#EBF1F5',
                'accent-green': '#E8F3EE',
                'accent-purple': '#F2EFF6',
                'accent-cream': '#FAF9F6',
                'border-subtle': '#F3F4F6',
            },
            fontFamily: {
                sans: ['Inter', 'sans-serif'],
            },
            boxShadow: {
                soft: '0 2px 15px -3px rgba(0, 0, 0, 0.03), 0 1px 4px -2px rgba(0, 0, 0, 0.02)',
            },
        },
    },
    plugins: [require('@tailwindcss/forms')],
};
